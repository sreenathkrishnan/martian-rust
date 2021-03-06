//! Martian adapter for Rust code

pub use failure::{format_err, Error};
use failure_derive::Fail;

use backtrace::Backtrace;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use log::{error, info};

use chrono::Local;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write as IoWrite;
use std::os::unix::io::FromRawFd;
use std::panic;
use std::path::Path;

mod metadata;
pub use metadata::*;

#[macro_use]
mod macros;
pub mod types;
pub use types::MartianFileType;

mod stage;
pub mod utils;
pub use stage::*;

pub mod mro;
pub use mro::*;

pub use log::LevelFilter;

pub mod prelude;

// Ways a stage can fail.
#[derive(Debug, Fail)]
pub enum StageError {
    // Controlled shutdown for known condition in data or config
    #[fail(display = "{}", message)]
    MartianExit { message: String },

    // Unexpected error
    #[fail(display = "{}", message)]
    PipelineError { message: String },
}

pub fn initialize(args: Vec<String>, log_file: &File) -> Result<Metadata, Error> {
    let mut md = Metadata::new(args, log_file);
    println!("got metadata: {:?}", md);
    md.update_jobinfo()?;

    Ok(md)
}

pub fn handle_stage_error(err: Error) {
    // Try to handle know StageError cases
    match &err.downcast::<StageError>() {
        &Ok(ref e) => {
            match e {
                &StageError::MartianExit { message: ref m } => {
                    let _ = write_errors(&format!("ASSERT: {}", m));
                }
                // No difference here at this point
                &StageError::PipelineError { message: ref m } => {
                    let _ = write_errors(&format!("ASSERT: {}", m));
                }
            }
        }
        &Err(ref e) => {
            let msg = format!("stage error:{}\n{}", e.as_fail(), e.backtrace());
            let _ = write_errors(&msg);
        }
    }
}

fn write_errors(msg: &str) -> Result<(), Error> {
    unsafe {
        let mut err_file = File::from_raw_fd(4);
        let _ = err_file.write(msg.as_bytes())?;
        Ok(())
    }
}

/// Log a panic to the martian output machinery
pub fn log_panic(panic: &panic::PanicInfo) {
    let payload = match panic.payload().downcast_ref::<String>() {
        Some(as_string) => format!("{}", as_string),
        None => format!("{:?}", panic.payload()),
    };

    let loc = panic.location().expect("location");
    let msg = format!("{}: {}\n{}", loc.file(), loc.line(), payload);

    let _ = write_errors(&msg);
}

fn setup_logging(log_file: &File, level: LevelFilter) {
    let base_config = fern::Dispatch::new().level(level);

    let logger_config = fern::Dispatch::new()
        .format(|out, msg, record| {
            let time_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            out.finish(format_args!("[{}][{}] {}", time_str, record.level(), msg))
        })
        .chain(log_file.try_clone().expect("couldn't open log file"))
        .chain(io::stdout());

    let cfg = base_config.chain(logger_config).apply();

    if let Err(e) = cfg {
        panic!("Failed to initialize global logger: {}", e);
    }
}

pub fn martian_main(
    args: Vec<String>,
    stage_map: HashMap<String, Box<RawMartianStage>>,
) -> Result<(), Error> {
    martian_main_with_log_level(args, stage_map, LevelFilter::Debug)
}

pub fn martian_main_with_log_level(
    args: Vec<String>,
    stage_map: HashMap<String, Box<RawMartianStage>>,
    level: LevelFilter,
) -> Result<(), Error> {
    info!("got args: {:?}", args);

    // The log file is opened by the monitor process and should never be closed by
    // the adapter.
    let log_file: File = unsafe { File::from_raw_fd(3) };

    // Hook rust logging up to Martian _log file
    setup_logging(&log_file, level);

    // setup Martian metadata
    let md = initialize(args, &log_file)?;

    // Get the stage implementation
    let stage = stage_map
        .get(&md.stage_name)
        .ok_or(failure::err_msg("couldn't find requested stage"))?;

    // Setup monitor thread -- this handles heartbeat & memory checking
    let stage_done = Arc::new(AtomicBool::new(false));

    // Setup panic hook. If a stage panics, we'll shutdown cleanly to martian
    let p = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::new();

        let thread = thread::current();
        let thread = thread.name().unwrap_or("unnamed");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        let msg = match info.location() {
            Some(location) => format!(
                "thread '{}' panicked at '{}': {}:{}{:?}",
                thread,
                msg,
                location.file(),
                location.line(),
                backtrace
            ),
            None => format!("thread '{}' panicked at '{}'{:?}", thread, msg, backtrace),
        };

        error!("{}", msg);
        let _ = write_errors(&msg);
        p(info);
    }));

    if md.stage_type == "split" {
        stage.split(md)?;
    } else if md.stage_type == "main" {
        stage.main(md)?;
    } else if md.stage_type == "join" {
        stage.join(md)?;
    } else {
        panic!("Unrecognized stage type");
    };

    stage_done.store(true, Ordering::Relaxed);
    Ok(())
}

const MRO_HEADER: &str = r#"
#
# Copyright (c) 10X Genomics, Inc. All rights reserved.
#
# WARNING: This file is auto-generated.
# DO NOT MODIFY THIS FILE DIRECTLY
#

"#;
pub fn martian_make_mro(
    file_name: Option<impl AsRef<Path>>,
    rewrite: bool,
    mro_registry: Vec<StageMro>,
) -> Result<(), Error> {
    if let Some(ref f) = file_name {
        let file_path = f.as_ref();
        if file_path.is_dir() {
            return Err(format_err!(
                "Error! Path {} is a directory!",
                file_path.display()
            ));
        }
        if file_path.exists() && !rewrite {
            return Err(format_err!(
                "File {} exists. You need to explicitly mention if it is okay to rewrite.",
                file_path.display()
            ));
        }
    }

    let mut filetype_header = FiletypeHeader::default();
    let mut mro_string = String::new();
    for stage_mro in mro_registry {
        filetype_header.add_stage(&stage_mro);
        writeln!(&mut mro_string, "{}", stage_mro)?;
    }

    let final_mro_string = format!("{}{}{}", MRO_HEADER, filetype_header, mro_string);
    match file_name {
        Some(f) => {
            let mut output = File::create(f)?;
            output.write(final_mro_string.as_bytes())?;
        }
        None => {
            println!("{}", final_mro_string);
        }
    }
    Ok(())
}

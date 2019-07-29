//!
//! This module is an attempt at defining traits that would help auto generate
//! mro definitions for a stage from a Rust stage. Previous attempt, named
//! `cargo-martian`, which is a compiler plugin works pretty good, but that
//! will work only in nightly builds and uses a fairly unstable part of the
//! compiler. Here we try to rework the same problem from a different angle
//! using procedural macros.
//!
//! # Mro syntax
//! Think about `mro` as an entity with the following properties
//! - Stage name
//! - List of stage/chunk inputs/outputs, each with a martian type
//! - Source for execution
//! - Attributes (mem_gb, vmem_gb, threads, volatile etc.)
//!

use crate::types::MartianVoid;
use crate::{MartianFileType, StageKind};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::string::ToString;

const MARTIAN_TOKENS: &'static [&'static str] = &[
    "in", "out", "stage", "volatile", "strict", "true", "split", "filetype", "src", "py", "comp",
    "retain",
];

/// Defines how an entity that denotes some part of the mro is displayed
pub trait MroDisplay {
    fn mro_string(&self, field_width: Option<usize>) -> String {
        match field_width {
            Some(width) => {
                let min_width = self.min_width();
                assert!(
                    width >= min_width,
                    format!("Need a minimum width of {:?}. Found {:?}", min_width, width)
                );
                self.mro_string_with_width(width)
            }
            None => self.mro_string_no_width(),
        }
    }
    fn min_width(&self) -> Self::FieldLen;
    fn mro_string_no_width(&self) -> String;
    fn mro_string_with_width(&self, Self::FieldLen) -> String;
}

/// A generic display impl for MroDisplay does not work due
/// to conflicting blanket impl. This is a simple macro to
/// write out the Display trait for MroDisplay
macro_rules! mro_display_to_display {
    ($type:ty) => {
        impl Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(&self.mro_string_no_width())
            }
        }
    };
}

macro_rules! usize_field_len {
    () => {
        type FieldLen = usize;
        fn min_width(&self) -> usize {
            self.mro_string_no_width().len()
        }
        fn mro_string_with_width(&self, field_width: usize) -> String {
            let value = self.mro_string_no_width();
            format!("{value:<width$}", value = value, width = field_width)
        }
    };
}

/// Primary data types in Martian world
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum MartianPrimaryType {
    Int,
    Float,
    Str,
    Bool,
    Map,
    Path,
    FileType(String),
}

impl MroDisplay for MartianPrimaryType {
    usize_field_len! {}
    fn mro_string_no_width(&self) -> String {
        let value = match *self {
            MartianPrimaryType::Int => "int",
            MartianPrimaryType::Float => "float",
            MartianPrimaryType::Str => "string",
            MartianPrimaryType::Bool => "bool",
            MartianPrimaryType::Map => "map",
            MartianPrimaryType::Path => "path",
            MartianPrimaryType::FileType(ref ext) => ext,
        };
        value.to_string()
    }
}

mro_display_to_display! {MartianPrimaryType}

/// Primary Data type + Arrays (which are derived from primary types)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum MartianType {
    Primary(MartianPrimaryType),
    Array(MartianPrimaryType),
}

impl MroDisplay for MartianType {
    usize_field_len! {}
    fn mro_string_no_width(&self) -> String {
        match *self {
            MartianType::Primary(ref primary) => primary.to_string(),
            MartianType::Array(ref primary) => format!("{}[]", primary.to_string()),
        }
    }
}
mro_display_to_display! {MartianType}

/// A trait that tells you how to convert a Rust data type to a
/// basic Martian type.
pub trait AsMartianPrimaryType {
    fn as_martian_primary_type() -> MartianPrimaryType;
}

/// A trait that defines how to convert this Rust type into an `MartianType`.
/// Not all rust types can be converted to an `MartianType`.
/// Not defined for
/// - Unit, the type of () in Rust.
/// - Unit Struct For example `struct Unit` or `PhantomData<T>`. It represents
///     a named value containing no data.
/// Any type which implements `AsMartianPrimaryType` also implements `AsMartianType`
/// It is stringly recommended not to extend any types with this trait, instead
/// use the `AsMartianPrimaryType` trait.
pub trait AsMartianType {
    fn as_martian_type() -> MartianType;
}

/// Macro for implementing `AsMartianPrimaryType` trait
macro_rules! impl_primary_mro_type {
    ($rust_type:ty, $mro_type:stmt) => {
        impl AsMartianPrimaryType for $rust_type {
            fn as_martian_primary_type() -> MartianPrimaryType {
                $mro_type
            }
        }
    };
}

impl_primary_mro_type!(i8, MartianPrimaryType::Int);
impl_primary_mro_type!(i16, MartianPrimaryType::Int);
impl_primary_mro_type!(i32, MartianPrimaryType::Int);
impl_primary_mro_type!(i64, MartianPrimaryType::Int);
impl_primary_mro_type!(i128, MartianPrimaryType::Int);
impl_primary_mro_type!(isize, MartianPrimaryType::Int);
impl_primary_mro_type!(u8, MartianPrimaryType::Int);
impl_primary_mro_type!(u16, MartianPrimaryType::Int);
impl_primary_mro_type!(u32, MartianPrimaryType::Int);
impl_primary_mro_type!(u64, MartianPrimaryType::Int);
impl_primary_mro_type!(u128, MartianPrimaryType::Int);
impl_primary_mro_type!(usize, MartianPrimaryType::Int);
impl_primary_mro_type!(bool, MartianPrimaryType::Bool);
impl_primary_mro_type!(f32, MartianPrimaryType::Float);
impl_primary_mro_type!(f64, MartianPrimaryType::Float);
impl_primary_mro_type!(char, MartianPrimaryType::Str);
impl_primary_mro_type!(String, MartianPrimaryType::Str);
impl_primary_mro_type!(&'static str, MartianPrimaryType::Str);
impl_primary_mro_type!(Path, MartianPrimaryType::Path);
impl_primary_mro_type!(PathBuf, MartianPrimaryType::Path);

impl<T: AsMartianPrimaryType> AsMartianType for T {
    fn as_martian_type() -> MartianType {
        MartianType::Primary(T::as_martian_primary_type())
    }
}

impl<T: AsMartianType> AsMartianType for Option<T> {
    fn as_martian_type() -> MartianType {
        // Any variable can be `null` in Martian
        T::as_martian_type()
    }
}

impl<T: AsMartianPrimaryType> AsMartianType for Vec<T> {
    fn as_martian_type() -> MartianType {
        MartianType::Array(T::as_martian_primary_type())
    }
}

impl<K: AsMartianPrimaryType, H> AsMartianType for HashSet<K, H> {
    fn as_martian_type() -> MartianType {
        MartianType::Array(K::as_martian_primary_type())
    }
}

impl<K, V, H> AsMartianPrimaryType for HashMap<K, V, H> {
    fn as_martian_primary_type() -> MartianPrimaryType {
        MartianPrimaryType::Map
    }
}

impl<T: MartianFileType> AsMartianPrimaryType for T {
    fn as_martian_primary_type() -> MartianPrimaryType {
        MartianPrimaryType::FileType(String::from(<T as MartianFileType>::extension()))
    }
}

/// Each variable that is listed in the mro along with it's type form
/// a `MroField`. For example, the following stage:
/// ```mro
/// stage SORT_ITEMS(
///     in  int[] unsorted,
///     in  bool  reverse,
///     out int[] sorted,
///     src comp  "my_stage martian sort_items",
/// )
/// ```
/// contains 3 `MroFields`
/// - MroField { name: unsorted, ty: MartianType::Array(MartianPrimaryType::Int)}
/// - MroField { name: reverse, ty: MartianType::Primary(MartianPrimaryType::Bool)}
/// - MroField { name: sorted, ty: MartianType::Array(MartianPrimaryType::Int)}
#[derive(Debug, Serialize, Clone, Deserialize, PartialEq, Eq)]
pub struct MroField {
    name: String,
    ty: MartianType,
}

impl MroDisplay for MroField {
    type FieldLen = usize; // Width of the type field
    fn mro_string_no_width(&self) -> String {
        format!("{ty} {name}", ty = self.ty.to_string(), name = &self.name)
    }
    fn min_width(&self) -> usize {
        self.ty.min_width()
    }

    fn mro_string_with_width(&self, field_width: usize) -> String {
        format!(
            "{ty} {name}",
            ty = self.ty.mro_string_with_width(field_width),
            name = &self.name
        )
    }
}

mro_display_to_display! {MroField}

impl MroField {
    pub fn new(name: impl ToString, ty: MartianType) -> Self {
        let field = MroField {
            name: name.to_string(),
            ty,
        };
        field.verify(); // No use case to resultify this so far
        field
    }
    // Check that name does not match any martian token.
    fn verify(&self) {
        for &token in MARTIAN_TOKENS.iter() {
            assert!(
                self.name != token,
                "Martian token {} cannot be used as field name",
                token
            );
        }
        assert!(!self.name.starts_with("__"));
    }
}

/// A trait that defines how to expand a struct into a list of `MroField`s
/// The `MartianStage` and `MartianMain` traits already has independent associated
/// types for stage/chunk inputs and outputs. If those associated types implement
/// this trait, then we can readily generate all the mro variables with the appropriate
/// type and put them at the right place (withing stage def or chunk def).
///
/// TODO : Auto derive for structs with named fields if all the fields implement `AsMartianType`
pub trait MartianStruct {
    /// How to convert this struct into a list of `MroField`s
    fn mro_fields() -> Vec<MroField>;
}

impl MartianStruct for MartianVoid {
    fn mro_fields() -> Vec<MroField> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Volatile {
    Strict,
}

impl FromStr for Volatile {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "strict" => Ok(Volatile::Strict),
            _ => Err(format!("Expected strict for volatile, Found {}", s)),
        }
    }
}

impl MroDisplay for Volatile {
    usize_field_len! {}
    fn mro_string_no_width(&self) -> String {
        match self {
            Volatile::Strict => "strict".into(),
        }
    }
}

mro_display_to_display! {Volatile}

const INDENT_TAB_WIDTH_FOR_MRO: usize = 4;
macro_rules! mro_using {
    ($($property:ident: $type:ty),*) => {
        /// Stuff that comes in the `using` section of a stage definition
        /// For example:
        /// ```mro
        /// using (
        ///     mem_gb  = 4,
        ///     threads = 16,
        /// )
        /// ```
        #[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
        pub struct MroUsing {
            $(pub $property: Option<$type>,)*
        }

        impl MroUsing {
            /// If all fields as None, return False
            pub fn need_using(&self) -> bool {
                !($(self.$property.is_none())&&*)
            }
        }

        /// Using section starts with `using (`
        /// followed by a line which has two configurable widths
        /// ```md
        /// using (
        ///          mem_gb = 1,
        ///      )
        /// <---><-->
        ///   w1  tab
        /// ```
        impl MroDisplay for MroUsing {
            type FieldLen = usize; // w1: See comments above
            fn min_width(&self) -> usize {
                0
            }

            fn mro_string_no_width(&self) -> String {
                self.mro_string_with_width(self.min_width())
            }

            fn mro_string_with_width(&self, w1: usize) -> String {
                let mut result = String::new();
                // If every field is None, return empty String
                if !self.need_using() {
                    return result;
                }
                let mut w2 = 0;
                $(if self.$property.is_some() {
                    w2 = std::cmp::max(w2, stringify!($property).len());
                })*
                writeln!(&mut result, "using (").unwrap();
                $(
                    if let Some($property) = self.$property {
                        writeln!(
                            &mut result,
                            "{blank:indent$}{key:<width$} = {value},",
                            blank = "",
                            indent = w1 + INDENT_TAB_WIDTH_FOR_MRO,
                            key=stringify!($property),
                            width=w2,
                            value=$property
                        ).unwrap()
                    }
                )*
                writeln!(&mut result, "{blank:indent$})", blank = "", indent = w1).unwrap();
                result
            }
        }
        mro_display_to_display! {MroUsing}
    };
}

mro_using! {mem_gb: i16, vmem_gb: i16, threads: i16, volatile: Volatile}

/// All the variables in the mro per stage/chunk inputs and outputs.
#[derive(Debug)]
pub struct StageVariables {
    pub stage_inputs: Vec<MroField>,
    pub stage_outputs: Vec<MroField>,
    pub chunk_inputs: Option<Vec<MroField>>,
    pub chunk_outputs: Option<Vec<MroField>>,
}

impl MroDisplay for StageVariables {
    type FieldLen = usize; // the indendation level
    fn min_width(&self) -> usize {
        0
    }
    fn mro_string_no_width(&self) -> String {
        self.mro_string_with_width(0)
    }
    fn mro_string_with_width(&self, field_width: usize) -> String {
        let mut result = String::new();
        // First compute the minimum width needed for the type column
        let mut min_w = 0;
        for field in &self.stage_inputs {
            min_w = std::cmp::max(min_w, field.ty.min_width())
        }
        for field in &self.stage_outputs {
            min_w = std::cmp::max(min_w, field.ty.min_width())
        }
        if let Some(ref chunk_inputs) = self.chunk_inputs {
            for field in chunk_inputs {
                min_w = std::cmp::max(min_w, field.ty.min_width())
            }
        }
        if let Some(ref chunk_outputs) = self.chunk_outputs {
            for field in chunk_outputs {
                min_w = std::cmp::max(min_w, field.ty.min_width())
            }
        }
        writeln!(&mut result, "(").unwrap();
        for field in &self.stage_inputs {
            writeln!(
                &mut result,
                "{blank:indent$}{key:<3} {ty} {name},",
                key = "in",
                blank = "",
                indent = INDENT_TAB_WIDTH_FOR_MRO,
                ty = field.ty.mro_string_with_width(min_w),
                name = field.name,
            )
            .unwrap();
        }
        for field in &self.stage_outputs {
            writeln!(
                &mut result,
                "{blank:indent$}{key:<3} {ty} {name},",
                key = "out",
                blank = "",
                indent = INDENT_TAB_WIDTH_FOR_MRO,
                ty = field.ty.mro_string_with_width(min_w),
                name = field.name,
            )
            .unwrap();
        }
        unimplemented!()
    }
}
///
#[derive(Debug)]
struct InAndOut {
    inputs: Vec<MroField>,
    outputs: Vec<MroField>,
}

/// All the data needed to create a stage definition mro.
///
#[derive(Debug)]
pub struct StageMro {
    stage_name: String,     // e.g CORRECT_BARCODES in `stage CORRECT_BARCODES(..)`
    adapter_name: String, // Martian adapter e.g `cr_slfe` in `src comp "cr_slfe martian correct_barcodes"
    stage_key: String, // Key used in the hashmap containing all stages e.g `correct_barcodes` in `src comp "cr_slfe martian correct_barcodes"
    stage_in_out: InAndOut, // Inputs and outputs of the stage
    chunk_in_out: Option<InAndOut>, // Inputs and outputs of the chunk. None indicates a stage with only a main
    using_attrs: MroUsing,          // Things coming under using
                                    // TODO: Retain
}

impl MroDisplay for StageMro {
    type FieldLen = usize;
}

// impl Stage {
//     fn to_mro_string(&self) -> String {
//         let mut mro = String::new();
//         let stage_name = to_shouty_snake_case(&self.name);
//         writeln!(&mut mro, "stage {}(", stage_name).unwrap();
//         for data in &self.stage_inputs {
//             writeln!(&mut mro, "    in {} {},", data.ty, data.name).unwrap();
//         }
//         for data in &self.stage_outputs {
//             writeln!(&mut mro, "    out {} {},", data.ty, data.name).unwrap();
//         }
//         let exec_name = to_snake_case(&self.name);
//         writeln!(&mut mro, "    src comp \"{} martian {}\",", self.binary, exec_name).unwrap();
//         // Split only if either chunk_inputs or chunk_outputs is not MartianVoid
//         if self.chunk_inputs.is_some() || self.chunk_outputs.is_some() {
//             writeln!(&mut mro, ") split (").unwrap();
//             for data in self.chunk_inputs.as_ref().unwrap() {
//                 writeln!(&mut mro, "    in {} {},", data.ty, data.name).unwrap();
//             }
//             for data in self.chunk_outputs.as_ref().unwrap() {
//                 writeln!(&mut mro, "    out {} {},", data.ty, data.name).unwrap();
//             }
//         }
//         writeln!(&mut mro, ") {}", self.using_attrs.to_mro_string()).unwrap();
//         mro
//     }

//     fn verify_and_minify(&mut self) {
//         // Make sure none of the fields have an invalid name
//         let verify_fields = |fields: &[MroField]| {
//             for f in fields {
//                 f.verify();
//             }
//         };
//         verify_fields(&self.stage_inputs);
//         verify_fields(&self.stage_outputs);

//         if self.chunk_inputs.is_none() && self.chunk_outputs.is_none() {
//             return;
//         }
//         verify_fields(self.chunk_inputs.as_ref().unwrap());
//         verify_fields(self.chunk_outputs.as_ref().unwrap());

//         // Do not allow the same field name in stage and chunk inputs
//         // O(mn) is good enough
//         for f_chunk in self.chunk_inputs.as_ref().unwrap() {
//             for f_stage in &self.stage_inputs {
//                 assert!(!(f_chunk.name==f_stage.name), "ERROR: Found identical field {} in stage and chunk inputs", f_chunk.name)
//             }
//         }

//         // If the same field name appears in stage and chunk outputs,
//         // make sure they are of the same type and remove the field
//         // from the chunk outputs
//         let mut min_chunk_outputs = Vec::new();
//         for f_chunk in self.chunk_outputs.as_ref().unwrap() {
//             let mut found = false;
//             for f_stage in &self.stage_outputs {
//                 if f_chunk.name==f_stage.name {
//                     found = true;
//                     assert!(f_chunk.ty==f_stage.ty, "ERROR: Identical field names in stage and chunk outputs need to have identical type. Offending field -> {}", f_chunk.name);
//                 }
//             }
//             if !found {
//                 min_chunk_outputs.push(f_chunk.clone());
//             }
//         }

//         self.chunk_outputs = Some(min_chunk_outputs);
//     }
// }

// impl std::fmt::Display for Stage {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "{}", self.to_mro_string())
//     }
// }

/// Can be auto generated using proc macro attribute
/// #[make_mro] on MartianMain or MartianStage
/// implementations if the associated types implement `MartianStruct`
pub trait MakeMro {
    // fn stage_mro() -> StageMro;
    fn stage_variables() -> StageVariables;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_martian_primary_type_display() {
        use MartianPrimaryType::*;
        assert_eq!(Int.mro_string_no_width(), "int");
        assert_eq!(Int.mro_string(Some(4)), "int ");
        assert_eq!(FileType("txt".into()).mro_string_with_width(5), "txt  ");
        assert_eq!(FileType("fastq.lz4".into()).mro_string(None), "fastq.lz4");
    }

    #[test]
    fn test_martian_type_display() {
        use MartianPrimaryType::*;
        use MartianType::*;
        assert_eq!(Primary(Int).mro_string_no_width(), "int");
        assert_eq!(Array(Int).mro_string(Some(7)), "int[]  ");
        assert_eq!(
            Array(FileType("txt".into())).mro_string_with_width(5),
            "txt[]"
        );
        assert_eq!(
            Primary(FileType("fastq.lz4".into())).mro_string(None),
            "fastq.lz4"
        );
    }

    #[test]
    fn test_volatile_parse() {
        assert_eq!("strict".parse::<Volatile>(), Ok(Volatile::Strict));
        assert!("foo".parse::<Volatile>().is_err());
    }

    #[test]
    fn test_volatile_display() {
        let vol = Volatile::Strict;
        assert_eq!(vol.mro_string(None), "strict");
        assert_eq!(vol.mro_string_no_width(), "strict");
        assert_eq!(vol.min_width(), 6);
        assert_eq!(vol.mro_string(Some(10)), "strict    ");
    }

    #[test]
    fn test_mro_using_display() {
        assert_eq!(
            MroUsing {
                mem_gb: Some(1),
                ..Default::default()
            }
            .to_string(),
            indoc!(
                "
                using (
                    mem_gb = 1,
                )
            "
            )
        );

        assert_eq!(
            MroUsing {
                mem_gb: Some(1),
                vmem_gb: Some(4),
                volatile: Some(Volatile::Strict),
                ..Default::default()
            }
            .mro_string_no_width(),
            indoc!(
                "
                using (
                    mem_gb   = 1,
                    vmem_gb  = 4,
                    volatile = strict,
                )
            "
            )
        );

        assert_eq!(
            MroUsing {
                threads: Some(2),
                ..Default::default()
            }
            .mro_string_with_width(8),
            indoc!(
                "
                using (
                            threads = 2,
                        )
            "
            )
        );
    }
    #[test]
    fn test_mro_using_need_using() {
        assert_eq!(MroUsing::default().need_using(), false);
        assert_eq!(
            MroUsing {
                mem_gb: Some(1),
                ..Default::default()
            }
            .need_using(),
            true
        );
        assert_eq!(
            MroUsing {
                mem_gb: Some(1),
                threads: Some(3),
                ..Default::default()
            }
            .need_using(),
            true
        );
    }
}

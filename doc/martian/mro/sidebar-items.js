initSidebarItems({"constant":[["MARTIAN_TOKENS",""]],"enum":[["MartianBlanketType","Primary Data type + Arrays (which are derived from primary types)"],["MartianPrimaryType","Primary data types in Martian world"],["Volatile",""]],"struct":[["FiletypeHeader","The list of filetypes we list at the top of the mro A simple wrapper around a HashSet of all file extensions."],["InAndOut","Input and outputs together"],["MroField","Each variable that is listed in the mro along with it's type form a `MroField`. For example, the following stage: `mro stage SORT_ITEMS(     in  int[] unsorted,     in  bool  reverse,     out int[] sorted,     src comp  \"my_stage martian sort_items\", ) ` contains 3 `MroFields` - MroField { name: unsorted, ty: MartianBlanketType::Array(MartianPrimaryType::Int)} - MroField { name: reverse, ty: MartianBlanketType::Primary(MartianPrimaryType::Bool)} - MroField { name: sorted, ty: MartianBlanketType::Array(MartianPrimaryType::Int)}"],["MroUsing","Stuff that comes in the `using` section of a stage definition For example: `mro using (     mem_gb  = 4,     threads = 16, ) `"],["StageMro","All the data needed to create a stage definition mro. TODO: Retain"]],"trait":[["AsMartianBlanketType","A trait that defines how to convert this Rust type into an `MartianBlanketType`. Not all rust types can be converted to an `MartianBlanketType`. Not defined for - Unit, the type of () in Rust. - Unit Struct For example `struct Unit` or `PhantomData<T>`. It represents     a named value containing no data. Any type which implements `AsMartianPrimaryType` also implements `AsMartianBlanketType` It is stringly recommended not to extend any types with this trait, instead use the `AsMartianPrimaryType` trait."],["AsMartianPrimaryType","A trait that tells you how to convert a Rust data type to a basic Martian type."],["MartianStruct","A trait that defines how to expand a struct into a list of `MroField`s The `MartianStage` and `MartianMain` traits already has independent associated types for stage/chunk inputs and outputs. If those associated types implement this trait, then we can readily generate all the mro variables with the appropriate type and put them at the right place (withing stage def or chunk def)."],["MroDisplay","Defines how an entity that denotes some part of the mro is displayed"],["MroMaker","Can be auto generated using proc macro attribute #[make_mro] on MartianMain or MartianStage implementations if the associated types implement `MartianStruct`"]]});
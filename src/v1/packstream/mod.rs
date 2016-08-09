pub mod marker;
pub mod serialize;
pub mod deserialize;
pub mod value;

pub use self::serialize::serialize;
pub use self::value::Value;

const STRUCTURE_IDENTIFIER: &'static str = "__STRUCTURE__";

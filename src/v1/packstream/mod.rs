pub mod marker;
pub mod serialize;
pub mod deserialize;
pub mod value;

pub use self::serialize::encode;
pub use self::deserialize::decode;
pub use self::value::Value;

const STRUCTURE_PREFIX: &'static str = "__STRUCTURE__";

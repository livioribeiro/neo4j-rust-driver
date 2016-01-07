use std::collections::BTreeMap;
use std::string;

pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(string::String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Structure {
        signature: u8,
        fields: Vec<Value>
    }
}

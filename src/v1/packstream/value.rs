use std::collections::BTreeMap;
use std::string;
use rustc_serialize::{Encodable, Encoder};

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(string::String),
    List(self::List),
    Map(self::Map),
}

pub type List = Vec<Value>;
pub type Map = BTreeMap<String, Value>;

impl Encodable for Value {
    fn encode<S: Encoder>(&self, e: &mut S) -> Result<(), S::Error> {
        match *self {
            Value::Null => e.emit_nil(),
            Value::Boolean(v) => v.encode(e),
            Value::Integer(v) => v.encode(e),
            Value::Float(v) => v.encode(e),
            Value::String(ref v) => v.encode(e),
            Value::List(ref v) => v.encode(e),
            Value::Map(ref v) => v.encode(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use ::v1::packstream::serialize::encode;
    use super::{Value, Map};

    #[test]
    fn serialize_null() {
        assert_eq!(encode(&()).unwrap(), encode(&Value::Null).unwrap());
    }

    #[test]
    fn serialize_bool() {
        assert_eq!(encode(&true).unwrap(), encode(&Value::Boolean(true)).unwrap());
        assert_eq!(encode(&false).unwrap(), encode(&Value::Boolean(false)).unwrap());
    }

    #[test]
    fn serialize_int() {
        assert_eq!(encode(&-9_223_372_036_854_775_808_i64).unwrap(), encode(&Value::Integer(-9_223_372_036_854_775_808_i64)).unwrap());
        assert_eq!(encode(&-2_147_483_648).unwrap(), encode(&Value::Integer(-2_147_483_648)).unwrap());
        assert_eq!(encode(&-32_768).unwrap(), encode(&Value::Integer(-32_768)).unwrap());
        assert_eq!(encode(&-128).unwrap(), encode(&Value::Integer(-128)).unwrap());
        assert_eq!(encode(&127).unwrap(), encode(&Value::Integer(127)).unwrap());
        assert_eq!(encode(&32_767).unwrap(), encode(&Value::Integer(32_767)).unwrap());
        assert_eq!(encode(&2_147_483_647).unwrap(), encode(&Value::Integer(2_147_483_647)).unwrap());
        assert_eq!(encode(&9_223_372_036_854_775_807_i64).unwrap(), encode(&Value::Integer(9_223_372_036_854_775_807_i64)).unwrap());
    }

    #[test]
    fn serialize_float() {
        assert_eq!(encode(&1.1).unwrap(), encode(&Value::Float(1.1)).unwrap());
        assert_eq!(encode(&-1.1).unwrap(), encode(&Value::Float(-1.1)).unwrap());
    }

    #[test]
    fn serialize_string() {
        assert_eq!(encode(&"abc".to_owned()).unwrap(), encode(&Value::String("abc".to_owned())).unwrap());
        assert_eq!(encode(&"abcdefghijklmnopqrstuvwxyz".to_owned()).unwrap(), encode(&Value::String("abcdefghijklmnopqrstuvwxyz".to_owned())).unwrap());
    }

    #[test]
    fn serialize_list() {
        assert_eq!(encode(&vec![1; 15]).unwrap(), encode(&Value::List(vec![Value::Integer(1); 15])).unwrap());
        assert_eq!(encode(&vec![1; 256]).unwrap(), encode(&Value::List(vec![Value::Integer(1); 256])).unwrap());
    }

    #[test]
    fn serialize_map() {
        let closure = |mut acc: Map, i: i64| { acc.insert(format!("{}", i), Value::Integer(i)); acc };
        let input = (0..15).fold(Map::new(), &closure);
        assert_eq!(encode(&input).unwrap(), encode(&Value::Map(input)).unwrap());

        let input = (0..256).fold(Map::new(), &closure);
        assert_eq!(encode(&input).unwrap(), encode(&Value::Map(input)).unwrap());
    }
}

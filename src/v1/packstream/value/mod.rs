use std::io::Read;
use std::collections::BTreeMap;
use std::convert::{From, Into};
use std::string;
use rustc_serialize::{Encodable, Encoder};

pub mod serialize;
mod builder;

use super::deserialize::DecodeResult;
pub use self::serialize::to_value;

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(string::String),
    List(self::List),
    Map(self::Map),
    Structure(u8, self::List)
}

pub type List = Vec<Value>;
pub type Map = BTreeMap<String, Value>;

impl Value {
    pub fn from_reader<R: Read>(reader: &mut R) -> DecodeResult<Self> {
        builder::from_reader(reader)
    }

    pub fn is_null(&self) -> bool {
        *self == Value::Null
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match *self {
            Value::Boolean(v) => Some(v),
            _ => None
        }
    }

    pub fn is_boolean(&self) -> bool {
        self.as_boolean().is_some()
    }

    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Integer(v) => Some(v),
            _ => None
        }
    }

    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }

    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Value::Float(v) => Some(v),
            _ => None
        }
    }

    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    pub fn as_string(&self) -> Option<&str> {
        match *self {
            Value::String(ref v) => Some(v),
            _ => None
        }
    }

    pub fn is_string(&self) -> bool {
        self.as_string().is_some()
    }

    pub fn as_list(&self) -> Option<&List> {
        match self {
            &Value::List(ref v) => Some(v),
            _ => None
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut List> {
        match self {
            &mut Value::List(ref mut v) => Some(v),
            _ => None
        }
    }

    pub fn is_list(&self) -> bool {
        self.as_list().is_some()
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            &Value::Map(ref v) => Some(v),
            _ => None
        }
    }

    pub fn as_map_mut(&mut self) -> Option<&mut Map> {
        match self {
            &mut Value::Map(ref mut v) => Some(v),
            _ => None
        }
    }

    pub fn is_map(&self) -> bool {
        self.as_map().is_some()
    }

    pub fn as_struct(&self) -> Option<(u8, &List)> {
        match self {
            &Value::Structure(s, ref v) => Some((s, v)),
            _ => None
        }
    }

    pub fn as_struct_mut(&mut self) -> Option<(&mut u8, &mut List)> {
        match self {
            &mut Value::Structure(ref mut s, ref mut v) => Some((s, v)),
            _ => None
        }
    }

    pub fn is_struct(&self) -> bool {
        self.as_struct().is_some()
    }
}

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
            Value::Structure(s, ref v) => {
                e.emit_struct(&format!("__STRUCTURE__{}", s as char), v.len(), |e| {
                    for f in v { try!(f.encode(e)); }
                    Ok(())
                })
            }
        }
    }
}

impl<T> From<Option<T>> for Value where T: Into<Value> {
    fn from(value: Option<T>) -> Self {
        value.map(|v| v.into()).unwrap_or(Value::Null)
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self { Value::Null }
}

impl From<bool> for Value {
    fn from(val: bool) -> Self { Value::Boolean(val) }
}

macro_rules! impl_from_int {
    ($($t:ty), +) => (
        $(impl From<$t> for Value {
            fn from(v: $t) -> Value { Value::Integer(v as i64) }
        })+
    )
}

impl_from_int!(usize, isize, u8, i8, u16, i16, u32, i32, u64, i64);

impl From<f32> for Value {
    fn from(val: f32) -> Self { Value::Float(val as f64) }
}

impl From<f64> for Value {
    fn from(val: f64) -> Self { Value::Float(val) }
}

impl<'a> From<&'a str> for Value {
    fn from(val: &'a str) -> Self { Value::String(val.to_owned()) }
}

impl From<String> for Value {
    fn from(val: String) -> Self { Value::String(val) }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(val: Vec<T>) -> Self {
        Value::List(val.into_iter().map(|i| i.into()).collect())
    }
}

impl<T: Into<Value>> From<BTreeMap<String, T>> for Value {
    fn from(val: BTreeMap<String, T>) -> Self {
        Value::Map(val.into_iter().fold(
            BTreeMap::<String, Value>::new(),
            |mut acc, (k, v)| { acc.insert(k, v.into()); acc }
        ))
    }
}

impl<'a, T: Into<Value>> From<BTreeMap<&'a str, T>> for Value {
    fn from(val: BTreeMap<&'a str, T>) -> Self {
        Value::Map(val.into_iter().fold(
            BTreeMap::<String, Value>::new(),
            |mut acc, (k, v)| { acc.insert(k.to_owned(), v.into()); acc }
        ))
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

    #[test]
    fn serialize_structure() {
        use rustc_serialize::{Encodable, Encoder};

        struct MyStruct {
            name: String,
            value: u32,
        }

        impl Encodable for MyStruct {
            fn encode<S: Encoder>(&self, e: &mut S) -> Result<(), S::Error> {
                try!(e.emit_struct("__STRUCTURE__\x22", 2, |_| Ok(())));
                try!(self.name.encode(e));
                self.value.encode(e)
            }
        }

        let input = MyStruct {
            name: "MyStruct".to_owned(),
            value: 42,
        };

        let expected = Value::Structure(
            0x22, vec![Value::String("MyStruct".to_owned()), Value::Integer(42)]
        );

        assert_eq!(encode(&input).unwrap(), encode(&expected).unwrap());
    }

    #[test]
    fn from_unit() {
        assert_eq!(Value::Null, Value::from(()));
    }

    #[test]
    fn from_bool() {
        assert_eq!(Value::Boolean(true), Value::from(true));
        assert_eq!(Value::Boolean(false), Value::from(false));
    }

    #[test]
    fn from_int() {
        assert_eq!(Value::Integer(42), Value::from(42usize));
        assert_eq!(Value::Integer(42), Value::from(42isize));

        assert_eq!(Value::Integer(42), Value::from(42u8));
        assert_eq!(Value::Integer(42), Value::from(42i8));

        assert_eq!(Value::Integer(42), Value::from(42u16));
        assert_eq!(Value::Integer(42), Value::from(42i16));

        assert_eq!(Value::Integer(42), Value::from(42u32));
        assert_eq!(Value::Integer(42), Value::from(42i32));

        assert_eq!(Value::Integer(42), Value::from(42u64));
        assert_eq!(Value::Integer(42), Value::from(42i64));
    }

    #[test]
    fn from_float() {
        assert_eq!(Value::Float(1.1f32 as f64), Value::from(1.1f32));
        assert_eq!(Value::Float(1.1), Value::from(1.1f64));
    }

    #[test]
    fn from_string() {
        assert_eq!(Value::String("abc".into()), Value::from("abc"));
        assert_eq!(Value::String("abc".into()), Value::from("abc".to_owned()));
    }

    #[test]
    fn from_vec() {
        assert_eq!(Value::List(vec![Value::Integer(1)]),
                   Value::from(vec![1]));
    }

    #[test]
    fn from_btreemap() {
        use ::std::collections::BTreeMap;

        let mut input: BTreeMap<&str, u32> = BTreeMap::new();
        input.insert("A", 1);

        let mut expected: BTreeMap<String, Value> = BTreeMap::new();
        expected.insert("A".to_owned(), Value::Integer(1));

        assert_eq!(Value::Map(expected.clone()), Value::from(input));

        let mut input: BTreeMap<String, u32> = BTreeMap::new();
        input.insert("A".to_owned(), 1);

        assert_eq!(Value::Map(expected.clone()), Value::from(input));
    }

    #[test]
    fn from_option_none() {
        let input: Option<()> = None;
        assert_eq!(Value::Null, Value::from(input));
    }

    #[test]
    fn from_option_some() {
        assert_eq!(Value::Boolean(true), Value::from(Some(true)));
        assert_eq!(Value::Integer(1), Value::from(Some(1)));
        assert_eq!(Value::Float(1.1), Value::from(Some(1.1)));
        assert_eq!(Value::String("abc".to_owned()), Value::from(Some("abc")));
        assert_eq!(Value::List(vec![Value::Integer(1)]), Value::from(Some(vec![1])));
    }
}

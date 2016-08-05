use std::io::Cursor;
use std::collections::BTreeMap;
use std::convert::{From, Into};
use std::string;

use serde::{Serializer, Serialize};
use byteorder::{WriteBytesExt, BigEndian};

use super::marker as m;

// pub mod serialize;
// mod builder;

// use super::deserialize::DecodeResult;
// pub use self::serialize::to_value;

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
    // pub fn from_reader<R: Read>(reader: &mut R) -> DecodeResult<Self> {
    //     builder::from_reader(reader)
    // }

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

impl Serialize for Value {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        match *self {
            Value::Null => serializer.serialize_unit(),
            Value::Boolean(v) => v.serialize(serializer),
            Value::Integer(v) => v.serialize(serializer),
            Value::Float(v) => v.serialize(serializer),
            Value::String(ref v) => v.serialize(serializer),
            Value::List(ref v) => v.serialize(serializer),
            Value::Map(ref v) => v.serialize(serializer),
            Value::Structure(signature, ref v) => {
                let mut cur: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(4));
                let len = v.len();
                if len <= m::USE_TINY_STRUCT {
                    cur.write_u8(m::TINY_STRUCT_NIBBLE | len as u8).unwrap();
                    cur.write_u8(signature).unwrap();
                } else if len <= m::USE_STRUCT_8 {
                    cur.write_u8(m::STRUCT_8).unwrap();
                    cur.write_u8(len as u8).unwrap();
                    cur.write_u8(signature).unwrap();
                // } else if len <= m::USE_STRUCT_16 {
                } else {
                    cur.write_u8(m::STRUCT_16).unwrap();
                    cur.write_u16::<BigEndian>(len as u16).unwrap();
                    cur.write_u8(signature).unwrap();
                }
                // } else {
                //     return Err(DesErr::InvalidStructureLength)
                // }

                let data = cur.into_inner();
                try!(serializer.serialize_bytes(data.as_ref()));

                for i in v.iter() {
                    try!(i.serialize(serializer));
                }

                Ok(())
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
    use serde::{Serializer, Serialize};
    use ::v1::packstream::serialize::serialize;
    use super::{Value, Map};
    use super::super::marker as m;

    #[test]
    fn serialize_null() {
        assert_eq!(serialize(&()).unwrap(), serialize(&Value::Null).unwrap());
    }

    #[test]
    fn serialize_bool() {
        assert_eq!(serialize(&true).unwrap(), serialize(&Value::Boolean(true)).unwrap());
        assert_eq!(serialize(&false).unwrap(), serialize(&Value::Boolean(false)).unwrap());
    }

    #[test]
    fn serialize_int() {
        assert_eq!(serialize(&-9_223_372_036_854_775_808_i64).unwrap(), serialize(&Value::Integer(-9_223_372_036_854_775_808_i64)).unwrap());
        assert_eq!(serialize(&-2_147_483_648).unwrap(), serialize(&Value::Integer(-2_147_483_648)).unwrap());
        assert_eq!(serialize(&-32_768).unwrap(), serialize(&Value::Integer(-32_768)).unwrap());
        assert_eq!(serialize(&-128).unwrap(), serialize(&Value::Integer(-128)).unwrap());
        assert_eq!(serialize(&127).unwrap(), serialize(&Value::Integer(127)).unwrap());
        assert_eq!(serialize(&32_767).unwrap(), serialize(&Value::Integer(32_767)).unwrap());
        assert_eq!(serialize(&2_147_483_647).unwrap(), serialize(&Value::Integer(2_147_483_647)).unwrap());
        assert_eq!(serialize(&9_223_372_036_854_775_807_i64).unwrap(), serialize(&Value::Integer(9_223_372_036_854_775_807_i64)).unwrap());
    }

    #[test]
    fn serialize_float() {
        assert_eq!(serialize(&1.1).unwrap(), serialize(&Value::Float(1.1)).unwrap());
        assert_eq!(serialize(&-1.1).unwrap(), serialize(&Value::Float(-1.1)).unwrap());
    }

    #[test]
    fn serialize_string() {
        assert_eq!(serialize(&"abc".to_owned()).unwrap(), serialize(&Value::String("abc".to_owned())).unwrap());
        assert_eq!(serialize(&"abcdefghijklmnopqrstuvwxyz".to_owned()).unwrap(), serialize(&Value::String("abcdefghijklmnopqrstuvwxyz".to_owned())).unwrap());
    }

    #[test]
    fn serialize_list() {
        assert_eq!(serialize(&vec![1; 15]).unwrap(), serialize(&Value::List(vec![Value::Integer(1); 15])).unwrap());
        assert_eq!(serialize(&vec![1; 256]).unwrap(), serialize(&Value::List(vec![Value::Integer(1); 256])).unwrap());
    }

    #[test]
    fn serialize_map() {
        let closure = |mut acc: Map, i: i64| { acc.insert(format!("{}", i), Value::Integer(i)); acc };

        let input = (0..15).fold(Map::new(), &closure);
        assert_eq!(serialize(&input).unwrap(), serialize(&Value::Map(input)).unwrap());

        let input = (0..256).fold(Map::new(), &closure);
        assert_eq!(serialize(&input).unwrap(), serialize(&Value::Map(input)).unwrap());
    }

    #[test]
    fn serialize_structure() {
        struct MyStruct {
            name: String,
            value: u32,
        }

        impl Serialize for MyStruct {
            fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
                where S: Serializer
            {
                let data = [m::TINY_STRUCT_NIBBLE | 0x02, 0x22];
                try!(serializer.serialize_bytes(&data));
                try!(self.name.serialize(serializer));
                self.value.serialize(serializer)
            }
        }

        let input = MyStruct {
            name: "MyStruct".to_owned(),
            value: 42,
        };

        let expected = Value::Structure(
            0x22, vec![Value::String("MyStruct".to_owned()), Value::Integer(42)]
        );

        assert_eq!(serialize(&expected).unwrap(), serialize(&input).unwrap());
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

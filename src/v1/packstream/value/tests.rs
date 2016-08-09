use serde::{Serializer, Serialize};
use ::v1::packstream::serialize::serialize;
use super::{Value, Map};
use super::super::marker as M;

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
    let input = Value::Structure(
        0x42, vec![Value::String("ABC".to_owned()), Value::Integer(1)]
    );

    let expected = vec![
        M::TINY_STRUCT_NIBBLE + 0x02, 0x42,
        M::TINY_STRING_NIBBLE + 0x03, b'A', b'B', b'C',
        0x01,
    ];

    assert_eq!(expected, serialize(&input).unwrap());
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

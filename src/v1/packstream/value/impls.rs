use std::collections::BTreeMap;

use super::Value;

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

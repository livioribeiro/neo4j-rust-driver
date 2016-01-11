use std::collections::BTreeMap;
use rustc_serialize::{Encodable, Encoder};

use super::Value;

// It is unlikely that the code here will fail, but if it does, it means that something really bad
// happened that is out of our control.
// Given this situation, the develpers decided that this function will return a Value instead of a
// Result<Value, Err> for convenience.
pub fn to_value<T: Encodable>(value: &T) -> Value {
    let mut encoder = ValueEncoder::new();
    value.encode(&mut encoder).expect("Something wrong happened while encoding data into `Value`");
    encoder.into_value()
}

struct ValueEncoder {
    stack: Vec<Value>
}

impl ValueEncoder {
    pub fn new() -> Self {
        ValueEncoder {
            stack: vec![],
        }
    }

    pub fn into_value(mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Null)
    }
}

impl Encoder for ValueEncoder {
    type Error = ();

    // Primitive types:
    fn emit_nil(&mut self) -> Result<(), Self::Error> {
        self.stack.push(Value::Null);
        Ok(())
    }

    fn emit_usize(&mut self, v: usize) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_u64(&mut self, v: u64) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_u32(&mut self, v: u32) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_u16(&mut self, v: u16) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_u8(&mut self, v: u8) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_isize(&mut self, v: isize) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_i64(&mut self, v: i64) -> Result<(), Self::Error> {
        self.stack.push(Value::Integer(v));
        Ok(())
    }

    fn emit_i32(&mut self, v: i32) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_i16(&mut self, v: i16) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_i8(&mut self, v: i8) -> Result<(), Self::Error> {
        self.emit_i64(v as i64)
    }

    fn emit_bool(&mut self, v: bool) -> Result<(), Self::Error> {
        self.stack.push(Value::Boolean(v));
        Ok(())
    }

    fn emit_f64(&mut self, v: f64) -> Result<(), Self::Error> {
        self.stack.push(Value::Float(v));
        Ok(())
    }

    fn emit_f32(&mut self, v: f32) -> Result<(), Self::Error> {
        self.emit_f64(v as f64)
    }

    fn emit_char(&mut self, v: char) -> Result<(), Self::Error> {
        let mut s = String::new();
        s.push(v);
        self.emit_str(&s)
    }

    fn emit_str(&mut self, v: &str) -> Result<(), Self::Error> {
        self.stack.push(Value::String(v.to_owned()));
        Ok(())
    }


    // Compound types:
    fn emit_enum<F>(&mut self, _: &str, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_enum_variant<F>(&mut self, v_name: &str,
                            _: usize,
                            len: usize,
                            f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        if len == 0 {
            self.emit_str(v_name)
        } else {
            self.emit_seq(len, f)
        }
    }

    fn emit_enum_variant_arg<F>(&mut self, _: usize, f: F)
                                -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_enum_struct_variant<F>(&mut self, v_name: &str,
                                   _: usize,
                                   len: usize,
                                   f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        if len == 0 {
            self.emit_str(v_name)
        } else {
            self.emit_map(len, f)
        }
    }

    fn emit_enum_struct_variant_field<F>(&mut self,
                                         f_name: &str,
                                         f_idx: usize,
                                         f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        try!(self.emit_str(f_name));
        self.emit_map_elt_val(f_idx, f)
    }

    fn emit_struct<F>(&mut self, _: &str, len: usize, f: F)
                      -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_map(len, f)
    }

    fn emit_struct_field<F>(&mut self, f_name: &str, _: usize, f: F)
                            -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        try!(self.emit_str(f_name));
        f(self)
    }

    fn emit_tuple<F>(&mut self, len: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq(len, f)
    }

    fn emit_tuple_arg<F>(&mut self, _: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_tuple_struct<F>(&mut self, _: &str, len: usize, f: F)
                            -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq(len, f)
    }

    fn emit_tuple_struct_arg<F>(&mut self, _: usize, f: F)
                                -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    // Specialized types:
    fn emit_option<F>(&mut self, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_option_none(&mut self) -> Result<(), Self::Error> {
        self.emit_nil()
    }

    fn emit_option_some<F>(&mut self, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_seq<F>(&mut self, len: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        try!(f(self));

        let values = {
            let mut values = vec![];
            for _ in 0..len {
                match self.stack.pop() {
                    Some(v) => values.push(v),
                    _ => panic!("Unexpected end of list")
                }
            }
            values.reverse();
            values

        };

        self.stack.push(Value::List(values));

        Ok(())
    }

    fn emit_seq_elt<F>(&mut self, _: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_map<F>(&mut self, len: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        try!(f(self));

        let len = len * 2;

        let values = {
            let mut cur_val: Option<Value> = None;
            let mut values: BTreeMap<String, Value> = BTreeMap::new();
            for i in 1..(len + 1) {
                match self.stack.pop() {
                    Some(Value::String(ref k)) if i % 2 == 0 => {
                        match cur_val.take() {
                            Some(v) => { values.insert(k.to_owned(), v); }
                            _ => panic!("Invalid map key")
                        }
                    },
                    Some(_) if i % 2 == 0 => panic!("Invalid map key"),
                    Some(v) => cur_val = Some(v),
                    _ => panic!("Unexpected end of map")
                }
            }
            values
        };

        self.stack.push(Value::Map(values));

        Ok(())
    }

    fn emit_map_elt_key<F>(&mut self, _: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_map_elt_val<F>(&mut self, _: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use super::to_value;
    use super::super::Value;

    #[test]
    fn serialize_nil() {
        let input = ();
        assert_eq!(Value::Null, to_value(&input));

        let input: Option<()> = None;
        assert_eq!(Value::Null, to_value(&input));
    }

    #[test]
    fn serialize_bool() {
        assert_eq!(Value::Boolean(true), to_value(&true));
        assert_eq!(Value::Boolean(false), to_value(&false));
    }

    #[test]
    fn encode_int() {
        assert_eq!(Value::Integer(1), to_value(&1));
    }

    #[test]
    fn encode_float() {
        assert_eq!(Value::Float(1.1), to_value(&1.1));
    }

    #[test]
    fn encode_string() {
        assert_eq!(Value::String("A".to_owned()), to_value(&"A"));
    }

    #[test]
    fn encode_vec() {
        let input = vec![1, 2, 3];
        let expected = Value::List(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]);

        assert_eq!(expected, to_value(&input));
    }

    #[test]
    fn encode_vec_of_vec() {
        let input = vec![vec![1], vec![2], vec![3]];
        let expected = Value::List(vec![
            Value::List(vec![Value::Integer(1)]),
            Value::List(vec![Value::Integer(2)]),
            Value::List(vec![Value::Integer(3)])]);

        assert_eq!(expected, to_value(&input));
    }

    #[test]
    fn encode_map() {
        let input = {
            let mut input: BTreeMap<String, u32> = BTreeMap::new();
            input.insert("A".to_owned(), 1);
            input.insert("B".to_owned(), 2);
            input.insert("C".to_owned(), 3);

            input
        };

        let expected = {
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();
            expected.insert("A".to_owned(), Value::Integer(1));
            expected.insert("B".to_owned(), Value::Integer(2));
            expected.insert("C".to_owned(), Value::Integer(3));

            Value::Map(expected)
        };

        assert_eq!(expected, to_value(&input));
    }

    #[test]
    fn encode_map_of_vec() {
        let input = {
            let mut input: BTreeMap<String, Vec<u32>> = BTreeMap::new();
            input.insert("A".to_owned(), vec![1]);
            input.insert("B".to_owned(), vec![2]);
            input.insert("C".to_owned(), vec![3]);

            input
        };

        let expected = {
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();
            expected.insert("A".to_owned(), Value::List(vec![Value::Integer(1)]));
            expected.insert("B".to_owned(), Value::List(vec![Value::Integer(2)]));
            expected.insert("C".to_owned(), Value::List(vec![Value::Integer(3)]));

            Value::Map(expected)
        };

        assert_eq!(expected, to_value(&input));
    }

    #[test]
    fn encode_map_of_vec_of_map() {
        let input = {
            let mut input: BTreeMap<String, Vec<BTreeMap<String, u32>>> = BTreeMap::new();

            let mut map1: BTreeMap<String, u32> = BTreeMap::new();
            map1.insert("X".to_owned(), 1);

            let mut map2: BTreeMap<String, u32> = BTreeMap::new();
            map2.insert("Y".to_owned(), 2);

            let mut map3: BTreeMap<String, u32> = BTreeMap::new();
            map3.insert("|".to_owned(), 3);


            input.insert("A".to_owned(), vec![map1]);
            input.insert("B".to_owned(), vec![map2]);
            input.insert("C".to_owned(), vec![map3]);

            input
        };

        let expected = {
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();

            let mut map1: BTreeMap<String, Value> = BTreeMap::new();
            map1.insert("X".to_owned(), Value::Integer(1));

            let mut map2: BTreeMap<String, Value> = BTreeMap::new();
            map2.insert("Y".to_owned(), Value::Integer(2));

            let mut map3: BTreeMap<String, Value> = BTreeMap::new();
            map3.insert("|".to_owned(), Value::Integer(3));

            expected.insert("A".to_owned(), Value::List(vec![Value::Map(map1)]));
            expected.insert("B".to_owned(), Value::List(vec![Value::Map(map2)]));
            expected.insert("C".to_owned(), Value::List(vec![Value::Map(map3)]));

            Value::Map(expected)
        };

        assert_eq!(expected, to_value(&input));
    }
}

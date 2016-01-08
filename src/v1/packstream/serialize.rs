#![allow(unused_variables)]

use std::io::prelude::*;
use std::io::Cursor;
use rustc_serialize::{Encodable, Encoder};
use byteorder::{self, WriteBytesExt, BigEndian};

use super::marker as m;

// #[derive(Debug)]
// pub struct EncoderError {
//     description: String,
//     cause: Option<Box<Error>>
// }
//
// impl From<io::Error> for EncoderError {
//     fn from(error: io::Error) -> Self {
//         EncoderError {
//             description: "IO Error".into(),
//             cause: Some(Box::new(error)),
//         }
//     }
// }
//
// impl fmt::Display for EncoderError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
//         let message = self.cause
//             .as_ref()
//             .map(|e| format!("{}", e))
//             .unwrap_or(self.description.clone());
//         write!(f, "Encoder Error: {}", message);
//         Ok(())
//     }
// }
//
// impl Error for EncoderError {
//     fn description(&self) -> &str {
//         &self.description
//     }
//
//     fn cause(&self) -> Option<&Error> {
//         self.cause.as_ref().map(|e| &**e)
//     }
// }

pub fn encode<T: Encodable>(object: &T) -> EncodeResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut encoder = PackstreamEncoder::new(&mut buf);
        try!(object.encode(&mut encoder));
    }
    Ok(buf.into_inner())
}

pub type EncodeResult<T> = Result<T, byteorder::Error>;

struct PackstreamEncoder<'a, W: Write + 'a> {
    writer: &'a mut W,
}

impl<'a, W: Write> PackstreamEncoder<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        PackstreamEncoder {
            writer: writer,
        }
    }
}

impl<'a, W: Write> Encoder for PackstreamEncoder<'a, W> {
    type Error = byteorder::Error;

    // Primitive types:
    fn emit_nil(&mut self) -> Result<(), Self::Error> {
        try!(self.writer.write_u8(m::NULL));
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
        if (v >= m::RANGE_POS_INT_64.0 && v <= m::RANGE_POS_INT_64.1)
            || (v >= m::RANGE_NEG_INT_64.0 && v <= m::RANGE_NEG_INT_64.1)
        {
            try!(self.writer.write_u8(m::INT_64));
            try!(self.writer.write_i64::<BigEndian>(v));
        } else if (v >= m::RANGE_POS_INT_32.0 && v <= m::RANGE_POS_INT_32.1)
            || (v >= m::RANGE_NEG_INT_32.0 && v <= m::RANGE_NEG_INT_32.1)
        {
            try!(self.writer.write_u8(m::INT_32));
            try!(self.writer.write_i32::<BigEndian>(v as i32));
        } else if (v >= m::RANGE_POS_INT_16.0 && v <= m::RANGE_POS_INT_16.1)
            || (v >= m::RANGE_NEG_INT_16.0 && v <= m::RANGE_NEG_INT_16.1)
        {
            try!(self.writer.write_u8(m::INT_16));
            try!(self.writer.write_i16::<BigEndian>(v as i16));
        } else if v >= m::RANGE_TINY_INT.0 && v <= m::RANGE_TINY_INT.1  {
            try!(self.writer.write_i8(v as i8));
        } else if v >= m::RANGE_NEG_INT_8.0 && v <= m::RANGE_NEG_INT_8.1 {
            try!(self.writer.write_u8(m::INT_8));
            try!(self.writer.write_i8(v as i8));
        }

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
        if v {
            try!(self.writer.write_u8(m::TRUE));
        } else {
            try!(self.writer.write_u8(m::FALSE));
        }

        Ok(())
    }

    fn emit_f64(&mut self, v: f64) -> Result<(), Self::Error> {
        try!(self.writer.write_u8(m::FLOAT));
        try!(self.writer.write_f64::<BigEndian>(v));

        Ok(())
    }

    fn emit_f32(&mut self, v: f32) -> Result<(), Self::Error> {
        self.emit_f64(v as f64)
    }

    fn emit_char(&mut self, v: char) -> Result<(), Self::Error> {
        try!(self.writer.write_u8(m::TINY_STRING_NIBBLE + 1));
        try!(self.writer.write_u8(v as u8));

        Ok(())
    }

    fn emit_str(&mut self, v: &str) -> Result<(), Self::Error> {
        let bytes = v.as_bytes();
        let size = bytes.len();

        if size <= m::USE_TINY_STRING {
            try!(self.writer.write_u8(m::TINY_STRING_NIBBLE + size as u8));
        } else if size <= m::USE_STRING_8 {
            try!(self.writer.write_u8(m::STRING_8));
            try!(self.writer.write_u8(size as u8));
        } else if size <= m::USE_STRING_16 {
            try!(self.writer.write_u8(m::STRING_16));
            try!(self.writer.write_u16::<BigEndian>(size as u16));
        } else if size <= m::USE_STRING_32 {
            try!(self.writer.write_u8(m::STRING_32));
            try!(self.writer.write_u32::<BigEndian>(size as u32));
        }

        try!(self.writer.write_all(bytes));

        Ok(())
    }


    // Compound types:
    fn emit_enum<F>(&mut self, name: &str, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn emit_enum_variant<F>(&mut self, v_name: &str,
                            v_id: usize,
                            len: usize,
                            f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn emit_enum_variant_arg<F>(&mut self, a_idx: usize, f: F)
                                -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn emit_enum_struct_variant<F>(&mut self, v_name: &str,
                                   v_id: usize,
                                   len: usize,
                                   f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn emit_enum_struct_variant_field<F>(&mut self,
                                         f_name: &str,
                                         f_idx: usize,
                                         f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn emit_struct<F>(&mut self, name: &str, len: usize, f: F)
                      -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn emit_struct_field<F>(&mut self, f_name: &str, f_idx: usize, f: F)
                            -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn emit_tuple<F>(&mut self, len: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq(len, f)
    }
    fn emit_tuple_arg<F>(&mut self, idx: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq_elt(idx, f)
    }

    fn emit_tuple_struct<F>(&mut self, name: &str, len: usize, f: F)
                            -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq(len, f)
    }
    fn emit_tuple_struct_arg<F>(&mut self, f_idx: usize, f: F)
                                -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq_elt(f_idx, f)
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

        if len <= m::USE_TINY_LIST as usize {
            try!(self.writer.write_u8(m::TINY_LIST_NIBBLE + len as u8));
        } else if len <= m::USE_LIST_8 as usize {
            try!(self.writer.write_u8(m::LIST_8));
            try!(self.writer.write_u8(len as u8));
        } else if len <= m::USE_LIST_16 as usize {
            try!(self.writer.write_u8(m::LIST_16));
            try!(self.writer.write_u16::<BigEndian>(len as u16));
        } else if len <= m::USE_LIST_32 as usize {
            try!(self.writer.write_u8(m::LIST_32));
            try!(self.writer.write_u32::<BigEndian>(len as u32));
        }

        f(self)
    }
    fn emit_seq_elt<F>(&mut self, idx: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_map<F>(&mut self, len: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

            if len <= m::USE_TINY_MAP as usize {
                try!(self.writer.write_u8(m::TINY_MAP_NIBBLE + len as u8));
            } else if len <= m::USE_MAP_8 as usize {
                try!(self.writer.write_u8(m::MAP_8));
                try!(self.writer.write_u8(len as u8));
            } else if len <= m::USE_MAP_16 as usize {
                try!(self.writer.write_u8(m::MAP_16));
                try!(self.writer.write_u16::<BigEndian>(len as u16));
            } else if len <= m::USE_MAP_32 as usize {
                try!(self.writer.write_u8(m::MAP_32));
                try!(self.writer.write_u32::<BigEndian>(len as u32));
            }

            f(self)
    }
    fn emit_map_elt_key<F>(&mut self, idx: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }
    fn emit_map_elt_val<F>(&mut self, idx: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::string::String;
    use super::encode;
    use ::v1::packstream::marker as m;

    #[test]
    fn serialize_nil() {
        let input: Option<()> = None;
        assert_eq!(vec![m::NULL], encode(&input).unwrap());
    }

    #[test]
    fn serialize_true() {
        assert_eq!(vec![m::TRUE], encode(&true).unwrap());
    }

    #[test]
    fn serialize_false() {
        assert_eq!(vec![m::FALSE], encode(&false).unwrap());
    }

    #[test]
    fn serialize_int64_positive() {
        let result = encode(&m::RANGE_POS_INT_64.1).unwrap();
        let expected = vec![m::INT_64, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int64_negative() {
        let result = encode(&m::RANGE_NEG_INT_64.0).unwrap();
        let expected = vec![m::INT_64, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int32_positive() {
        let result = encode(&m::RANGE_POS_INT_32.1).unwrap();
        let expected = vec![m::INT_32, 0x7F, 0xFF, 0xFF, 0xFF];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int32_negative() {
        let result = encode(&m::RANGE_NEG_INT_32.0).unwrap();
        let expected = vec![m::INT_32, 0x80, 0x00, 0x00, 0x00];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int16_positive() {
        let result = encode(&m::RANGE_POS_INT_16.1).unwrap();
        let expected = vec![m::INT_16, 0x7F, 0xFF];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int16_negative() {
        let result = encode(&m::RANGE_NEG_INT_16.0).unwrap();
        let expected = vec![m::INT_16, 0x80, 0x00];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int8_min() {
        let result = encode(&m::RANGE_NEG_INT_8.0).unwrap();
        let expected = vec![m::INT_8, 0x80];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_int8_max() {
        let result = encode(&m::RANGE_NEG_INT_8.1).unwrap();
        let expected = vec![m::INT_8, 0xEF];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tiny_int_min() {
        let result = encode(&m::RANGE_TINY_INT.0).unwrap();
        let expected = vec![0xF0];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tiny_int_max() {
        let result = encode(&m::RANGE_TINY_INT.1).unwrap();
        let expected = vec![0x7F];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_float_positive() {
        let result = encode(&1.1).unwrap();
        let expected = vec![m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_float_negative() {
        let result = encode(&-1.1).unwrap();
        let expected = vec![m::FLOAT, 0xBF, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A];
        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_string32() {
        let size = 70_000;
        let input = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::STRING_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, _| { acc.push(b'A'); acc }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_string16() {
        let size = 5_000;
        let input = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::STRING_16, 0x13, 0x88],
            |mut acc, _| { acc.push(b'A'); acc }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_string8() {
        let size = 200;
        let input = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::STRING_8, 0xC8],
            |mut acc, _| { acc.push(b'A'); acc }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tiny_string() {
        for marker in 0x80..0x8F {
            let size = marker - m::TINY_STRING_NIBBLE;
            let input = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });

            let result = encode(&input).unwrap();
            let expected = (0..size).fold(
                vec![marker],
                |mut acc, _| { acc.push(b'A'); acc }
            );

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn serialize_char() {
        for c in b'A'..b'Z' {
            let result: Vec<u8> = encode(&(c as char)).unwrap();
            let expected = vec![0x81, c];

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn serialize_list32() {
        let size = 70_000;
        let input = vec![1; size];

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::LIST_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, _| { acc.push(0x01); acc }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_list16() {
        let size = 5_000;
        let input = vec![1; size];

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::LIST_16, 0x13, 0x88],
            |mut acc, _| { acc.push(0x01); acc }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_list8() {
        let size = 200;
        let input = vec![1; size];

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::LIST_8, 0xC8],
            |mut acc, _| { acc.push(0x01); acc }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tiny_list() {
        for marker in 0x90..0x9F {
            let size = (marker - m::TINY_LIST_NIBBLE) as usize;
            let input = vec![1; size];

            let result = encode(&input).unwrap();
            let expected = (0..size).fold(
                vec![marker],
                |mut acc, _| { acc.push(0x01); acc }
            );

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn serialize_list_of_string() {
        let size = 3;
        let input = vec!["abcdefghijklmnopqrstuvwxyz"; size];

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_LIST_NIBBLE + size as u8,
                            m::STRING_8, 0x1A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                            0x77, 0x78, 0x79, 0x7A,
                            m::STRING_8, 0x1A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                            0x77, 0x78, 0x79, 0x7A,
                            m::STRING_8, 0x1A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                            0x77, 0x78, 0x79, 0x7A];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_list_of_int() {
        let size = 3;
        let input = vec![32_000; size];

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_LIST_NIBBLE + size as u8,
                            m::INT_16, 0x7D, 0x00,
                            m::INT_16, 0x7D, 0x00,
                            m::INT_16, 0x7D, 0x00];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_list_of_float() {
        let size = 3;
        let input = vec![1.1; size];

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_LIST_NIBBLE + size as u8,
                            m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                            m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                            m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_list_of_bool() {
        let size = 4;
        let input = vec![true, false, true, false];

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_LIST_NIBBLE + size as u8,
                            m::TRUE, m::FALSE, m::TRUE, m::FALSE];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tuple() {
        let size = 3;
        let input = (1, 1.1, "A");

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_LIST_NIBBLE + size as u8,
                            0x01,
                            m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                            m::TINY_STRING_NIBBLE + 1, 0x41];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map32() {
        let size = 70_000;
        let input = (0..size).fold(
            BTreeMap::<String, u32>::new(),
            |mut acc, i| { acc.insert(format!("A{}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = input.keys().fold(
            vec![m::MAP_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, i| {
                acc.append(&mut encode(&i).unwrap());
                acc.push(0x01);
                acc
            }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map16() {
        let size = 5_000;
        let input = (0..size).fold(
            BTreeMap::<String, u32>::new(),
            |mut acc, i| { acc.insert(format!("A{}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = input.keys().fold(
            vec![m::MAP_16, 0x13, 0x88],
            |mut acc, i| {
                acc.append(&mut encode(&i).unwrap());
                acc.push(0x01);
                acc
            }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map8() {
        let size = 200;
        let input = (0..size).fold(
            BTreeMap::<String, u32>::new(),
            |mut acc, i| { acc.insert(format!("A{}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = input.keys().fold(
            vec![m::MAP_8, 0xC8],
            |mut acc, i| {
                acc.append(&mut encode(&i).unwrap());
                acc.push(0x01);
                acc
            }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tiny_map() {
        let size = 3;
        let input = (0..size).fold(
            BTreeMap::<String, u32>::new(),
            |mut acc, i| { acc.insert(format!("A{}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = input.keys().fold(
            vec![m::TINY_MAP_NIBBLE + size],
            |mut acc, i| {
                acc.append(&mut encode(&i).unwrap());
                acc.push(0x01);
                acc
            }
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map_of_string() {
        let size = 3;
        let input = {
            let mut input: BTreeMap<&'static str, &'static str> = BTreeMap::new();
            input.insert("A", "abcdefghijklmnopqrstuvwxyz");
            input.insert("B", "abcdefghijklmnopqrstuvwxyz");
            input.insert("C", "abcdefghijklmnopqrstuvwxyz");
            input
        };

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_MAP_NIBBLE + size,
                            0x81, 0x41,
                            m::STRING_8, 0x1A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                            0x77, 0x78, 0x79, 0x7A,
                            0x81, 0x42,
                            m::STRING_8, 0x1A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                            0x77, 0x78, 0x79, 0x7A,
                            0x81, 0x43,
                            m::STRING_8, 0x1A, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                            0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
                            0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
                            0x77, 0x78, 0x79, 0x7A];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map_of_int() {
        let size = 3;
        let input = {
            let mut input: BTreeMap<&'static str, u32> = BTreeMap::new();
            input.insert("A", 32_000);
            input.insert("B", 32_000);
            input.insert("C", 32_000);
            input
        };

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_MAP_NIBBLE + size,
                            0x81, 0x41, m::INT_16, 0x7D, 0x00,
                            0x81, 0x42, m::INT_16, 0x7D, 0x00,
                            0x81, 0x43, m::INT_16, 0x7D, 0x00];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map_of_float() {
        let size = 3;
        let input = {
            let mut input: BTreeMap<&'static str, f64> = BTreeMap::new();
            input.insert("A", 1.1);
            input.insert("B", 1.1);
            input.insert("C", 1.1);
            input
        };

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_MAP_NIBBLE + size,
                            0x81, 0x41, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                            0x81, 0x42, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                            0x81, 0x43, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_map_of_bool() {
        let size = 4;
        let input = {
            let mut input: BTreeMap<&'static str, bool> = BTreeMap::new();
            input.insert("A", true);
            input.insert("B", false);
            input.insert("C", true);
            input.insert("D", false);
            input
        };

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_MAP_NIBBLE + size,
                            0x81, 0x41, m::TRUE,
                            0x81, 0x42, m::FALSE,
                            0x81, 0x43, m::TRUE,
                            0x81, 0x44, m::FALSE];

        assert_eq!(expected, result);
    }
}

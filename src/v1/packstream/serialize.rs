use std::error::Error;
use std::fmt;
use std::io::prelude::*;
use std::io::{self, Cursor};
use rustc_serialize::{Encodable, Encoder};
use byteorder::{WriteBytesExt, BigEndian};

use super::marker as m;
use super::STRUCTURE_PREFIX;

pub fn encode<T: Encodable>(object: &T) -> EncodeResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut encoder = PackstreamEncoder::new(&mut buf);
        try!(object.encode(&mut encoder));
    }
    Ok(buf.into_inner())
}

#[derive(Debug)]
pub enum EncoderError {
    IoError(io::Error),
    InvalidStructureLength,
}

impl Error for EncoderError {
    fn description(&self) -> &str { "encoder error" }
}

impl fmt::Display for EncoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl From<io::Error> for EncoderError {
    fn from(error: io::Error) -> Self {
        EncoderError::IoError(error)
    }
}

pub type EncodeResult<T> = Result<T, EncoderError>;

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
    type Error = EncoderError;

    // Primitive types:
    fn emit_nil(&mut self) -> Result<(), Self::Error> {
        try!(self.writer.write_u8(m::NULL));
        Ok(())
    }

    fn emit_usize(&mut self, v: usize) -> Result<(), Self::Error> {
        self.emit_u64(v as u64)
    }

    fn emit_u64(&mut self, v: u64) -> Result<(), Self::Error> {
        if v >= m::RANGE_POS_INT_64.0 as u64 && v <= m::RANGE_POS_INT_64.1 as u64 {
            try!(self.writer.write_u8(m::INT_64));
            try!(self.writer.write_u64::<BigEndian>(v));
        } else if v >= m::RANGE_POS_INT_32.0 as u64 && v <= m::RANGE_POS_INT_32.1 as u64 {
            try!(self.writer.write_u8(m::INT_32));
            try!(self.writer.write_u32::<BigEndian>(v as u32));
        } else if v >= m::RANGE_POS_INT_16.0 as u64 && v <= m::RANGE_POS_INT_16.1 as u64 {
            try!(self.writer.write_u8(m::INT_16));
            try!(self.writer.write_u16::<BigEndian>(v as u16));
        } else if v <= m::RANGE_TINY_INT.1 as u64 {
            try!(self.writer.write_u8(v as u8));
        }

        Ok(())
    }

    fn emit_u32(&mut self, v: u32) -> Result<(), Self::Error> {
        self.emit_u64(v as u64)
    }

    fn emit_u16(&mut self, v: u16) -> Result<(), Self::Error> {
        self.emit_u64(v as u64)
    }

    fn emit_u8(&mut self, v: u8) -> Result<(), Self::Error> {
        self.emit_u64(v as u64)
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
        try!(self.writer.write_u8(m::TINY_STRING_NIBBLE | 0x01));
        try!(self.writer.write_u8(v as u8));

        Ok(())
    }

    fn emit_str(&mut self, v: &str) -> Result<(), Self::Error> {
        let bytes = v.as_bytes();
        let size = bytes.len();

        if size <= m::USE_TINY_STRING {
            try!(self.writer.write_u8(m::TINY_STRING_NIBBLE | size as u8));
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
            try!(self.writer.write_u8(m::TINY_MAP_NIBBLE | 0x01));
            try!(self.emit_str(v_name));
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

    fn emit_struct<F>(&mut self, name: &str, len: usize, f: F)
                      -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        if name.starts_with(STRUCTURE_PREFIX) {
            debug_assert!(name.len() == STRUCTURE_PREFIX.len() + 1, "Invalid structure name: '{}'", name);
            // it is garanteed that the name is not empty
            let signature = *name.as_bytes().last().unwrap();

            if len <= m::USE_TINY_STRUCT {
                try!(self.writer.write_u8(m::TINY_STRUCT_NIBBLE | len as u8));
                try!(self.writer.write_u8(signature));
            } else if len <= m::USE_STRUCT_8 {
                try!(self.writer.write_u8(m::STRUCT_8));
                try!(self.writer.write_u8(signature));
                try!(self.writer.write_u8(len as u8));
            } else if len <= m::USE_STRUCT_16 {
                try!(self.writer.write_u8(m::STRUCT_16));
                try!(self.writer.write_u8(signature));
                try!(self.writer.write_u16::<BigEndian>(len as u16));
            } else {
                return Err(EncoderError::InvalidStructureLength)
            }

            f(self)
        } else {
            self.emit_map(len, f)
        }
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

    fn emit_tuple_arg<F>(&mut self, idx: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        self.emit_seq_elt(idx, f)
    }

    fn emit_tuple_struct<F>(&mut self, _: &str, len: usize, f: F)
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
            try!(self.writer.write_u8(m::TINY_LIST_NIBBLE | len as u8));
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
    fn emit_seq_elt<F>(&mut self, _: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

        f(self)
    }

    fn emit_map<F>(&mut self, len: usize, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Self) -> Result<(), Self::Error> {

            if len <= m::USE_TINY_MAP as usize {
                try!(self.writer.write_u8(m::TINY_MAP_NIBBLE | len as u8));
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
    use std::string::String;
    use super::encode;
    use ::v1::packstream::marker as m;

    #[test]
    fn serialize_nil() {
        let input = ();
        assert_eq!(vec![m::NULL], encode(&input).unwrap());

        let input: Option<()> = None;
        assert_eq!(vec![m::NULL], encode(&input).unwrap());
    }

    #[test]
    fn serialize_bool() {
        assert_eq!(vec![m::TRUE], encode(&true).unwrap());
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
            |mut acc, i| { acc.insert(format!("{:05}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::MAP_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, i| {
                let b1 = 48 + ((i % 100000) / 10000) as u8;
                let b2 = 48 + ((i % 10000) / 1000) as u8;
                let b3 = 48 + ((i % 1000) / 100) as u8;
                let b4 = 48 + ((i % 100) / 10) as u8;
                let b5 = 48 + (i % 10) as u8;
                acc.extend([0x85, b1, b2, b3, b4, b5, 0x01].iter());
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
            |mut acc, i| { acc.insert(format!("{:04}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::MAP_16, 0x13, 0x88],
            |mut acc, i| {
                let b1 = 48 + ((i % 10000) / 1000) as u8;
                let b2 = 48 + ((i % 1000) / 100) as u8;
                let b3 = 48 + ((i % 100) / 10) as u8;
                let b4 = 48 + (i % 10) as u8;
                acc.extend([0x84, b1, b2, b3, b4, 0x01].iter());
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
            |mut acc, i| { acc.insert(format!("{:03}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::MAP_8, 0xC8],
            |mut acc, i| {
                let b1 = 48 + ((i % 1000) / 100) as u8;
                let b2 = 48 + ((i % 100) / 10) as u8;
                let b3 = 48 + (i % 10) as u8;
                acc.extend([0x83, b1, b2, b3, 0x01].iter());
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
            |mut acc, i| { acc.insert(format!("{}", i), 1); acc }
        );

        let result = encode(&input).unwrap();
        let expected = (0..size).fold(
            vec![m::TINY_MAP_NIBBLE + size],
            |mut acc, i| {
                acc.extend([0x81, 0x30 + i].iter());
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
                            0x81, 0x43, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A];

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

    #[test]
    fn serialize_struct16() {
        #[derive(RustcEncodable)]
        #[allow(non_snake_case)]
        struct MyStruct {
            A001: u16, A002: u16, A003: u16, A004: u16, A005: u16, A006: u16, A007: u16, A008: u16,
            A009: u16, A010: u16, A011: u16, A012: u16, A013: u16, A014: u16, A015: u16, A016: u16,
            A017: u16, A018: u16, A019: u16, A020: u16, A021: u16, A022: u16, A023: u16, A024: u16,
            A025: u16, A026: u16, A027: u16, A028: u16, A029: u16, A030: u16, A031: u16, A032: u16,
            A033: u16, A034: u16, A035: u16, A036: u16, A037: u16, A038: u16, A039: u16, A040: u16,
            A041: u16, A042: u16, A043: u16, A044: u16, A045: u16, A046: u16, A047: u16, A048: u16,
            A049: u16, A050: u16, A051: u16, A052: u16, A053: u16, A054: u16, A055: u16, A056: u16,
            A057: u16, A058: u16, A059: u16, A060: u16, A061: u16, A062: u16, A063: u16, A064: u16,
            A065: u16, A066: u16, A067: u16, A068: u16, A069: u16, A070: u16, A071: u16, A072: u16,
            A073: u16, A074: u16, A075: u16, A076: u16, A077: u16, A078: u16, A079: u16, A080: u16,
            A081: u16, A082: u16, A083: u16, A084: u16, A085: u16, A086: u16, A087: u16, A088: u16,
            A089: u16, A090: u16, A091: u16, A092: u16, A093: u16, A094: u16, A095: u16, A096: u16,
            A097: u16, A098: u16, A099: u16, A100: u16, A101: u16, A102: u16, A103: u16, A104: u16,
            A105: u16, A106: u16, A107: u16, A108: u16, A109: u16, A110: u16, A111: u16, A112: u16,
            A113: u16, A114: u16, A115: u16, A116: u16, A117: u16, A118: u16, A119: u16, A120: u16,
            A121: u16, A122: u16, A123: u16, A124: u16, A125: u16, A126: u16, A127: u16, A128: u16,
            A129: u16, A130: u16, A131: u16, A132: u16, A133: u16, A134: u16, A135: u16, A136: u16,
            A137: u16, A138: u16, A139: u16, A140: u16, A141: u16, A142: u16, A143: u16, A144: u16,
            A145: u16, A146: u16, A147: u16, A148: u16, A149: u16, A150: u16, A151: u16, A152: u16,
            A153: u16, A154: u16, A155: u16, A156: u16, A157: u16, A158: u16, A159: u16, A160: u16,
            A161: u16, A162: u16, A163: u16, A164: u16, A165: u16, A166: u16, A167: u16, A168: u16,
            A169: u16, A170: u16, A171: u16, A172: u16, A173: u16, A174: u16, A175: u16, A176: u16,
            A177: u16, A178: u16, A179: u16, A180: u16, A181: u16, A182: u16, A183: u16, A184: u16,
            A185: u16, A186: u16, A187: u16, A188: u16, A189: u16, A190: u16, A191: u16, A192: u16,
            A193: u16, A194: u16, A195: u16, A196: u16, A197: u16, A198: u16, A199: u16, A200: u16,
            A201: u16, A202: u16, A203: u16, A204: u16, A205: u16, A206: u16, A207: u16, A208: u16,
            A209: u16, A210: u16, A211: u16, A212: u16, A213: u16, A214: u16, A215: u16, A216: u16,
            A217: u16, A218: u16, A219: u16, A220: u16, A221: u16, A222: u16, A223: u16, A224: u16,
            A225: u16, A226: u16, A227: u16, A228: u16, A229: u16, A230: u16, A231: u16, A232: u16,
            A233: u16, A234: u16, A235: u16, A236: u16, A237: u16, A238: u16, A239: u16, A240: u16,
            A241: u16, A242: u16, A243: u16, A244: u16, A245: u16, A246: u16, A247: u16, A248: u16,
            A249: u16, A250: u16, A251: u16, A252: u16, A253: u16, A254: u16, A255: u16, A256: u16,
        }

        let input = MyStruct {
            A001: 1, A002: 1, A003: 1, A004: 1, A005: 1, A006: 1, A007: 1, A008: 1,
            A009: 1, A010: 1, A011: 1, A012: 1, A013: 1, A014: 1, A015: 1, A016: 1,
            A017: 1, A018: 1, A019: 1, A020: 1, A021: 1, A022: 1, A023: 1, A024: 1,
            A025: 1, A026: 1, A027: 1, A028: 1, A029: 1, A030: 1, A031: 1, A032: 1,
            A033: 1, A034: 1, A035: 1, A036: 1, A037: 1, A038: 1, A039: 1, A040: 1,
            A041: 1, A042: 1, A043: 1, A044: 1, A045: 1, A046: 1, A047: 1, A048: 1,
            A049: 1, A050: 1, A051: 1, A052: 1, A053: 1, A054: 1, A055: 1, A056: 1,
            A057: 1, A058: 1, A059: 1, A060: 1, A061: 1, A062: 1, A063: 1, A064: 1,
            A065: 1, A066: 1, A067: 1, A068: 1, A069: 1, A070: 1, A071: 1, A072: 1,
            A073: 1, A074: 1, A075: 1, A076: 1, A077: 1, A078: 1, A079: 1, A080: 1,
            A081: 1, A082: 1, A083: 1, A084: 1, A085: 1, A086: 1, A087: 1, A088: 1,
            A089: 1, A090: 1, A091: 1, A092: 1, A093: 1, A094: 1, A095: 1, A096: 1,
            A097: 1, A098: 1, A099: 1, A100: 1, A101: 1, A102: 1, A103: 1, A104: 1,
            A105: 1, A106: 1, A107: 1, A108: 1, A109: 1, A110: 1, A111: 1, A112: 1,
            A113: 1, A114: 1, A115: 1, A116: 1, A117: 1, A118: 1, A119: 1, A120: 1,
            A121: 1, A122: 1, A123: 1, A124: 1, A125: 1, A126: 1, A127: 1, A128: 1,
            A129: 1, A130: 1, A131: 1, A132: 1, A133: 1, A134: 1, A135: 1, A136: 1,
            A137: 1, A138: 1, A139: 1, A140: 1, A141: 1, A142: 1, A143: 1, A144: 1,
            A145: 1, A146: 1, A147: 1, A148: 1, A149: 1, A150: 1, A151: 1, A152: 1,
            A153: 1, A154: 1, A155: 1, A156: 1, A157: 1, A158: 1, A159: 1, A160: 1,
            A161: 1, A162: 1, A163: 1, A164: 1, A165: 1, A166: 1, A167: 1, A168: 1,
            A169: 1, A170: 1, A171: 1, A172: 1, A173: 1, A174: 1, A175: 1, A176: 1,
            A177: 1, A178: 1, A179: 1, A180: 1, A181: 1, A182: 1, A183: 1, A184: 1,
            A185: 1, A186: 1, A187: 1, A188: 1, A189: 1, A190: 1, A191: 1, A192: 1,
            A193: 1, A194: 1, A195: 1, A196: 1, A197: 1, A198: 1, A199: 1, A200: 1,
            A201: 1, A202: 1, A203: 1, A204: 1, A205: 1, A206: 1, A207: 1, A208: 1,
            A209: 1, A210: 1, A211: 1, A212: 1, A213: 1, A214: 1, A215: 1, A216: 1,
            A217: 1, A218: 1, A219: 1, A220: 1, A221: 1, A222: 1, A223: 1, A224: 1,
            A225: 1, A226: 1, A227: 1, A228: 1, A229: 1, A230: 1, A231: 1, A232: 1,
            A233: 1, A234: 1, A235: 1, A236: 1, A237: 1, A238: 1, A239: 1, A240: 1,
            A241: 1, A242: 1, A243: 1, A244: 1, A245: 1, A246: 1, A247: 1, A248: 1,
            A249: 1, A250: 1, A251: 1, A252: 1, A253: 1, A254: 1, A255: 1, A256: 1,
        };

        let result = encode(&input).unwrap();

        let expected = vec![m::MAP_16, 0x01, 0x00,
            0x84, 0x41, 0x30, 0x30, 0x31, 0x01, 0x84, 0x41, 0x30, 0x30, 0x32, 0x01, 0x84, 0x41, 0x30, 0x30, 0x33, 0x01, 0x84, 0x41, 0x30, 0x30, 0x34, 0x01, 0x84, 0x41, 0x30, 0x30, 0x35, 0x01, 0x84, 0x41, 0x30, 0x30, 0x36, 0x01, 0x84, 0x41, 0x30, 0x30, 0x37, 0x01, 0x84, 0x41, 0x30, 0x30, 0x38, 0x01,
            0x84, 0x41, 0x30, 0x30, 0x39, 0x01, 0x84, 0x41, 0x30, 0x31, 0x30, 0x01, 0x84, 0x41, 0x30, 0x31, 0x31, 0x01, 0x84, 0x41, 0x30, 0x31, 0x32, 0x01, 0x84, 0x41, 0x30, 0x31, 0x33, 0x01, 0x84, 0x41, 0x30, 0x31, 0x34, 0x01, 0x84, 0x41, 0x30, 0x31, 0x35, 0x01, 0x84, 0x41, 0x30, 0x31, 0x36, 0x01,
            0x84, 0x41, 0x30, 0x31, 0x37, 0x01, 0x84, 0x41, 0x30, 0x31, 0x38, 0x01, 0x84, 0x41, 0x30, 0x31, 0x39, 0x01, 0x84, 0x41, 0x30, 0x32, 0x30, 0x01, 0x84, 0x41, 0x30, 0x32, 0x31, 0x01, 0x84, 0x41, 0x30, 0x32, 0x32, 0x01, 0x84, 0x41, 0x30, 0x32, 0x33, 0x01, 0x84, 0x41, 0x30, 0x32, 0x34, 0x01,
            0x84, 0x41, 0x30, 0x32, 0x35, 0x01, 0x84, 0x41, 0x30, 0x32, 0x36, 0x01, 0x84, 0x41, 0x30, 0x32, 0x37, 0x01, 0x84, 0x41, 0x30, 0x32, 0x38, 0x01, 0x84, 0x41, 0x30, 0x32, 0x39, 0x01, 0x84, 0x41, 0x30, 0x33, 0x30, 0x01, 0x84, 0x41, 0x30, 0x33, 0x31, 0x01, 0x84, 0x41, 0x30, 0x33, 0x32, 0x01,
            0x84, 0x41, 0x30, 0x33, 0x33, 0x01, 0x84, 0x41, 0x30, 0x33, 0x34, 0x01, 0x84, 0x41, 0x30, 0x33, 0x35, 0x01, 0x84, 0x41, 0x30, 0x33, 0x36, 0x01, 0x84, 0x41, 0x30, 0x33, 0x37, 0x01, 0x84, 0x41, 0x30, 0x33, 0x38, 0x01, 0x84, 0x41, 0x30, 0x33, 0x39, 0x01, 0x84, 0x41, 0x30, 0x34, 0x30, 0x01,
            0x84, 0x41, 0x30, 0x34, 0x31, 0x01, 0x84, 0x41, 0x30, 0x34, 0x32, 0x01, 0x84, 0x41, 0x30, 0x34, 0x33, 0x01, 0x84, 0x41, 0x30, 0x34, 0x34, 0x01, 0x84, 0x41, 0x30, 0x34, 0x35, 0x01, 0x84, 0x41, 0x30, 0x34, 0x36, 0x01, 0x84, 0x41, 0x30, 0x34, 0x37, 0x01, 0x84, 0x41, 0x30, 0x34, 0x38, 0x01,
            0x84, 0x41, 0x30, 0x34, 0x39, 0x01, 0x84, 0x41, 0x30, 0x35, 0x30, 0x01, 0x84, 0x41, 0x30, 0x35, 0x31, 0x01, 0x84, 0x41, 0x30, 0x35, 0x32, 0x01, 0x84, 0x41, 0x30, 0x35, 0x33, 0x01, 0x84, 0x41, 0x30, 0x35, 0x34, 0x01, 0x84, 0x41, 0x30, 0x35, 0x35, 0x01, 0x84, 0x41, 0x30, 0x35, 0x36, 0x01,
            0x84, 0x41, 0x30, 0x35, 0x37, 0x01, 0x84, 0x41, 0x30, 0x35, 0x38, 0x01, 0x84, 0x41, 0x30, 0x35, 0x39, 0x01, 0x84, 0x41, 0x30, 0x36, 0x30, 0x01, 0x84, 0x41, 0x30, 0x36, 0x31, 0x01, 0x84, 0x41, 0x30, 0x36, 0x32, 0x01, 0x84, 0x41, 0x30, 0x36, 0x33, 0x01, 0x84, 0x41, 0x30, 0x36, 0x34, 0x01,
            0x84, 0x41, 0x30, 0x36, 0x35, 0x01, 0x84, 0x41, 0x30, 0x36, 0x36, 0x01, 0x84, 0x41, 0x30, 0x36, 0x37, 0x01, 0x84, 0x41, 0x30, 0x36, 0x38, 0x01, 0x84, 0x41, 0x30, 0x36, 0x39, 0x01, 0x84, 0x41, 0x30, 0x37, 0x30, 0x01, 0x84, 0x41, 0x30, 0x37, 0x31, 0x01, 0x84, 0x41, 0x30, 0x37, 0x32, 0x01,
            0x84, 0x41, 0x30, 0x37, 0x33, 0x01, 0x84, 0x41, 0x30, 0x37, 0x34, 0x01, 0x84, 0x41, 0x30, 0x37, 0x35, 0x01, 0x84, 0x41, 0x30, 0x37, 0x36, 0x01, 0x84, 0x41, 0x30, 0x37, 0x37, 0x01, 0x84, 0x41, 0x30, 0x37, 0x38, 0x01, 0x84, 0x41, 0x30, 0x37, 0x39, 0x01, 0x84, 0x41, 0x30, 0x38, 0x30, 0x01,
            0x84, 0x41, 0x30, 0x38, 0x31, 0x01, 0x84, 0x41, 0x30, 0x38, 0x32, 0x01, 0x84, 0x41, 0x30, 0x38, 0x33, 0x01, 0x84, 0x41, 0x30, 0x38, 0x34, 0x01, 0x84, 0x41, 0x30, 0x38, 0x35, 0x01, 0x84, 0x41, 0x30, 0x38, 0x36, 0x01, 0x84, 0x41, 0x30, 0x38, 0x37, 0x01, 0x84, 0x41, 0x30, 0x38, 0x38, 0x01,
            0x84, 0x41, 0x30, 0x38, 0x39, 0x01, 0x84, 0x41, 0x30, 0x39, 0x30, 0x01, 0x84, 0x41, 0x30, 0x39, 0x31, 0x01, 0x84, 0x41, 0x30, 0x39, 0x32, 0x01, 0x84, 0x41, 0x30, 0x39, 0x33, 0x01, 0x84, 0x41, 0x30, 0x39, 0x34, 0x01, 0x84, 0x41, 0x30, 0x39, 0x35, 0x01, 0x84, 0x41, 0x30, 0x39, 0x36, 0x01,
            0x84, 0x41, 0x30, 0x39, 0x37, 0x01, 0x84, 0x41, 0x30, 0x39, 0x38, 0x01, 0x84, 0x41, 0x30, 0x39, 0x39, 0x01, 0x84, 0x41, 0x31, 0x30, 0x30, 0x01, 0x84, 0x41, 0x31, 0x30, 0x31, 0x01, 0x84, 0x41, 0x31, 0x30, 0x32, 0x01, 0x84, 0x41, 0x31, 0x30, 0x33, 0x01, 0x84, 0x41, 0x31, 0x30, 0x34, 0x01,
            0x84, 0x41, 0x31, 0x30, 0x35, 0x01, 0x84, 0x41, 0x31, 0x30, 0x36, 0x01, 0x84, 0x41, 0x31, 0x30, 0x37, 0x01, 0x84, 0x41, 0x31, 0x30, 0x38, 0x01, 0x84, 0x41, 0x31, 0x30, 0x39, 0x01, 0x84, 0x41, 0x31, 0x31, 0x30, 0x01, 0x84, 0x41, 0x31, 0x31, 0x31, 0x01, 0x84, 0x41, 0x31, 0x31, 0x32, 0x01,
            0x84, 0x41, 0x31, 0x31, 0x33, 0x01, 0x84, 0x41, 0x31, 0x31, 0x34, 0x01, 0x84, 0x41, 0x31, 0x31, 0x35, 0x01, 0x84, 0x41, 0x31, 0x31, 0x36, 0x01, 0x84, 0x41, 0x31, 0x31, 0x37, 0x01, 0x84, 0x41, 0x31, 0x31, 0x38, 0x01, 0x84, 0x41, 0x31, 0x31, 0x39, 0x01, 0x84, 0x41, 0x31, 0x32, 0x30, 0x01,
            0x84, 0x41, 0x31, 0x32, 0x31, 0x01, 0x84, 0x41, 0x31, 0x32, 0x32, 0x01, 0x84, 0x41, 0x31, 0x32, 0x33, 0x01, 0x84, 0x41, 0x31, 0x32, 0x34, 0x01, 0x84, 0x41, 0x31, 0x32, 0x35, 0x01, 0x84, 0x41, 0x31, 0x32, 0x36, 0x01, 0x84, 0x41, 0x31, 0x32, 0x37, 0x01, 0x84, 0x41, 0x31, 0x32, 0x38, 0x01,
            0x84, 0x41, 0x31, 0x32, 0x39, 0x01, 0x84, 0x41, 0x31, 0x33, 0x30, 0x01, 0x84, 0x41, 0x31, 0x33, 0x31, 0x01, 0x84, 0x41, 0x31, 0x33, 0x32, 0x01, 0x84, 0x41, 0x31, 0x33, 0x33, 0x01, 0x84, 0x41, 0x31, 0x33, 0x34, 0x01, 0x84, 0x41, 0x31, 0x33, 0x35, 0x01, 0x84, 0x41, 0x31, 0x33, 0x36, 0x01,
            0x84, 0x41, 0x31, 0x33, 0x37, 0x01, 0x84, 0x41, 0x31, 0x33, 0x38, 0x01, 0x84, 0x41, 0x31, 0x33, 0x39, 0x01, 0x84, 0x41, 0x31, 0x34, 0x30, 0x01, 0x84, 0x41, 0x31, 0x34, 0x31, 0x01, 0x84, 0x41, 0x31, 0x34, 0x32, 0x01, 0x84, 0x41, 0x31, 0x34, 0x33, 0x01, 0x84, 0x41, 0x31, 0x34, 0x34, 0x01,
            0x84, 0x41, 0x31, 0x34, 0x35, 0x01, 0x84, 0x41, 0x31, 0x34, 0x36, 0x01, 0x84, 0x41, 0x31, 0x34, 0x37, 0x01, 0x84, 0x41, 0x31, 0x34, 0x38, 0x01, 0x84, 0x41, 0x31, 0x34, 0x39, 0x01, 0x84, 0x41, 0x31, 0x35, 0x30, 0x01, 0x84, 0x41, 0x31, 0x35, 0x31, 0x01, 0x84, 0x41, 0x31, 0x35, 0x32, 0x01,
            0x84, 0x41, 0x31, 0x35, 0x33, 0x01, 0x84, 0x41, 0x31, 0x35, 0x34, 0x01, 0x84, 0x41, 0x31, 0x35, 0x35, 0x01, 0x84, 0x41, 0x31, 0x35, 0x36, 0x01, 0x84, 0x41, 0x31, 0x35, 0x37, 0x01, 0x84, 0x41, 0x31, 0x35, 0x38, 0x01, 0x84, 0x41, 0x31, 0x35, 0x39, 0x01, 0x84, 0x41, 0x31, 0x36, 0x30, 0x01,
            0x84, 0x41, 0x31, 0x36, 0x31, 0x01, 0x84, 0x41, 0x31, 0x36, 0x32, 0x01, 0x84, 0x41, 0x31, 0x36, 0x33, 0x01, 0x84, 0x41, 0x31, 0x36, 0x34, 0x01, 0x84, 0x41, 0x31, 0x36, 0x35, 0x01, 0x84, 0x41, 0x31, 0x36, 0x36, 0x01, 0x84, 0x41, 0x31, 0x36, 0x37, 0x01, 0x84, 0x41, 0x31, 0x36, 0x38, 0x01,
            0x84, 0x41, 0x31, 0x36, 0x39, 0x01, 0x84, 0x41, 0x31, 0x37, 0x30, 0x01, 0x84, 0x41, 0x31, 0x37, 0x31, 0x01, 0x84, 0x41, 0x31, 0x37, 0x32, 0x01, 0x84, 0x41, 0x31, 0x37, 0x33, 0x01, 0x84, 0x41, 0x31, 0x37, 0x34, 0x01, 0x84, 0x41, 0x31, 0x37, 0x35, 0x01, 0x84, 0x41, 0x31, 0x37, 0x36, 0x01,
            0x84, 0x41, 0x31, 0x37, 0x37, 0x01, 0x84, 0x41, 0x31, 0x37, 0x38, 0x01, 0x84, 0x41, 0x31, 0x37, 0x39, 0x01, 0x84, 0x41, 0x31, 0x38, 0x30, 0x01, 0x84, 0x41, 0x31, 0x38, 0x31, 0x01, 0x84, 0x41, 0x31, 0x38, 0x32, 0x01, 0x84, 0x41, 0x31, 0x38, 0x33, 0x01, 0x84, 0x41, 0x31, 0x38, 0x34, 0x01,
            0x84, 0x41, 0x31, 0x38, 0x35, 0x01, 0x84, 0x41, 0x31, 0x38, 0x36, 0x01, 0x84, 0x41, 0x31, 0x38, 0x37, 0x01, 0x84, 0x41, 0x31, 0x38, 0x38, 0x01, 0x84, 0x41, 0x31, 0x38, 0x39, 0x01, 0x84, 0x41, 0x31, 0x39, 0x30, 0x01, 0x84, 0x41, 0x31, 0x39, 0x31, 0x01, 0x84, 0x41, 0x31, 0x39, 0x32, 0x01,
            0x84, 0x41, 0x31, 0x39, 0x33, 0x01, 0x84, 0x41, 0x31, 0x39, 0x34, 0x01, 0x84, 0x41, 0x31, 0x39, 0x35, 0x01, 0x84, 0x41, 0x31, 0x39, 0x36, 0x01, 0x84, 0x41, 0x31, 0x39, 0x37, 0x01, 0x84, 0x41, 0x31, 0x39, 0x38, 0x01, 0x84, 0x41, 0x31, 0x39, 0x39, 0x01, 0x84, 0x41, 0x32, 0x30, 0x30, 0x01,
            0x84, 0x41, 0x32, 0x30, 0x31, 0x01, 0x84, 0x41, 0x32, 0x30, 0x32, 0x01, 0x84, 0x41, 0x32, 0x30, 0x33, 0x01, 0x84, 0x41, 0x32, 0x30, 0x34, 0x01, 0x84, 0x41, 0x32, 0x30, 0x35, 0x01, 0x84, 0x41, 0x32, 0x30, 0x36, 0x01, 0x84, 0x41, 0x32, 0x30, 0x37, 0x01, 0x84, 0x41, 0x32, 0x30, 0x38, 0x01,
            0x84, 0x41, 0x32, 0x30, 0x39, 0x01, 0x84, 0x41, 0x32, 0x31, 0x30, 0x01, 0x84, 0x41, 0x32, 0x31, 0x31, 0x01, 0x84, 0x41, 0x32, 0x31, 0x32, 0x01, 0x84, 0x41, 0x32, 0x31, 0x33, 0x01, 0x84, 0x41, 0x32, 0x31, 0x34, 0x01, 0x84, 0x41, 0x32, 0x31, 0x35, 0x01, 0x84, 0x41, 0x32, 0x31, 0x36, 0x01,
            0x84, 0x41, 0x32, 0x31, 0x37, 0x01, 0x84, 0x41, 0x32, 0x31, 0x38, 0x01, 0x84, 0x41, 0x32, 0x31, 0x39, 0x01, 0x84, 0x41, 0x32, 0x32, 0x30, 0x01, 0x84, 0x41, 0x32, 0x32, 0x31, 0x01, 0x84, 0x41, 0x32, 0x32, 0x32, 0x01, 0x84, 0x41, 0x32, 0x32, 0x33, 0x01, 0x84, 0x41, 0x32, 0x32, 0x34, 0x01,
            0x84, 0x41, 0x32, 0x32, 0x35, 0x01, 0x84, 0x41, 0x32, 0x32, 0x36, 0x01, 0x84, 0x41, 0x32, 0x32, 0x37, 0x01, 0x84, 0x41, 0x32, 0x32, 0x38, 0x01, 0x84, 0x41, 0x32, 0x32, 0x39, 0x01, 0x84, 0x41, 0x32, 0x33, 0x30, 0x01, 0x84, 0x41, 0x32, 0x33, 0x31, 0x01, 0x84, 0x41, 0x32, 0x33, 0x32, 0x01,
            0x84, 0x41, 0x32, 0x33, 0x33, 0x01, 0x84, 0x41, 0x32, 0x33, 0x34, 0x01, 0x84, 0x41, 0x32, 0x33, 0x35, 0x01, 0x84, 0x41, 0x32, 0x33, 0x36, 0x01, 0x84, 0x41, 0x32, 0x33, 0x37, 0x01, 0x84, 0x41, 0x32, 0x33, 0x38, 0x01, 0x84, 0x41, 0x32, 0x33, 0x39, 0x01, 0x84, 0x41, 0x32, 0x34, 0x30, 0x01,
            0x84, 0x41, 0x32, 0x34, 0x31, 0x01, 0x84, 0x41, 0x32, 0x34, 0x32, 0x01, 0x84, 0x41, 0x32, 0x34, 0x33, 0x01, 0x84, 0x41, 0x32, 0x34, 0x34, 0x01, 0x84, 0x41, 0x32, 0x34, 0x35, 0x01, 0x84, 0x41, 0x32, 0x34, 0x36, 0x01, 0x84, 0x41, 0x32, 0x34, 0x37, 0x01, 0x84, 0x41, 0x32, 0x34, 0x38, 0x01,
            0x84, 0x41, 0x32, 0x34, 0x39, 0x01, 0x84, 0x41, 0x32, 0x35, 0x30, 0x01, 0x84, 0x41, 0x32, 0x35, 0x31, 0x01, 0x84, 0x41, 0x32, 0x35, 0x32, 0x01, 0x84, 0x41, 0x32, 0x35, 0x33, 0x01, 0x84, 0x41, 0x32, 0x35, 0x34, 0x01, 0x84, 0x41, 0x32, 0x35, 0x35, 0x01, 0x84, 0x41, 0x32, 0x35, 0x36, 0x01,
        ];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_struct8() {
        let size = 16;

        #[derive(RustcEncodable)]
        #[allow(non_snake_case)]
        struct MyStruct {
            A: u16, B: u16, C: u16, D: u16,
            E: u16, F: u16, G: u16, H: u16,
            I: u16, J: u16, K: u16, L: u16,
            M: u16, N: u16, O: u16, P: u16,
        }

        let input = MyStruct {
            A: 1, B: 1, C: 1, D: 1,
            E: 1, F: 1, G: 1, H: 1,
            I: 1, J: 1, K: 1, L: 1,
            M: 1, N: 1, O: 1, P: 1,
        };

        let result = encode(&input).unwrap();
        let expected = vec![m::MAP_8, size,
                            0x81, 0x41, 0x01, 0x81, 0x42, 0x01, 0x81, 0x43, 0x01, 0x81, 0x44, 0x01,
                            0x81, 0x45, 0x01, 0x81, 0x46, 0x01, 0x81, 0x47, 0x01, 0x81, 0x48, 0x01,
                            0x81, 0x49, 0x01, 0x81, 0x4A, 0x01, 0x81, 0x4B, 0x01, 0x81, 0x4C, 0x01,
                            0x81, 0x4D, 0x01, 0x81, 0x4E, 0x01, 0x81, 0x4F, 0x01, 0x81, 0x50, 0x01];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_tiny_struct() {
        let size = 3;

        #[derive(RustcEncodable)]
        #[allow(non_snake_case)]
        struct MyStruct {
            A: u32,
            B: f64,
            C: &'static str,
        }

        let input = MyStruct {
            A: 1,
            B: 1.1,
            C: "C",
        };

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_MAP_NIBBLE + size,
                            0x81, 0x41, 0x01,
                            0x81, 0x42, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                            0x81, 0x43, 0x81, 0x43];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_enum() {
        #[derive(RustcEncodable)]
        enum MyEnum {
            A,
        }

        let input = MyEnum::A;

        let result = encode(&input).unwrap();
        let expected = vec![0x81, 0x41];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_enum_tuple_variant() {
        #[derive(RustcEncodable)]
        enum MyEnum {
            A(u16, u16),
        }

        let input = MyEnum::A(1, 2);

        let result = encode(&input).unwrap();
        let expected = vec![m::TINY_MAP_NIBBLE + 0x01,
                            0x81, 0x41,
                            0x92, 0x01, 0x02];

        assert_eq!(expected, result);
    }

    // #[test]
    // fn serialize_enum_struct_variant() {
    //     let size = 2;
    //
    //     #[derive(RustcEncodable)]
    //     #[allow(non_snake_case)]
    //     enum MyEnum {
    //         A { A: u16, B: u16 },
    //     }
    //
    //     let input = MyEnum::A { A: 1, B: 2 };
    //
    //     let result = encode(&input).unwrap();
    //     let expected = vec![m::TINY_MAP_NIBBLE + size,
    //                         0x81, 0x41, 0x01,
    //                         0x81, 0x42, 0x02];
    //
    //     assert_eq!(expected, result);
    // }
}

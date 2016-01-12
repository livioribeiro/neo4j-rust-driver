use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io::prelude::*;
use std::io::{self, BufReader, Cursor};
use std::string;
use rustc_serialize::{Decodable, Decoder};
use byteorder::{self, ReadBytesExt, BigEndian};

use super::marker as m;

fn is_tiny_int(b: u8) -> bool { b >> 7 == 0x00 || b >> 4 == m::TINY_INT_NEG_NIBBLE >> 4 }
fn is_tiny_string(b: u8) -> bool { b >> 4 == m::TINY_STRING_NIBBLE >> 4 }
fn is_tiny_list(b: u8) -> bool { b >> 4 == m::TINY_LIST_NIBBLE >> 4 }
fn is_tiny_map(b: u8) -> bool { b >> 4 == m::TINY_MAP_NIBBLE >> 4 }
fn is_tiny_struct(b: u8) -> bool { b >> 4 == m::TINY_STRUCT_NIBBLE >> 4 }

fn is_int8_or_lesser(b: u8) -> bool {
    b == m::INT_8 || is_tiny_int(b)
}

fn is_int16_or_lesser(b: u8) -> bool {
    b == m::INT_16 || is_int8_or_lesser(b)
}

fn is_int32_or_lesser(b: u8) -> bool {
    b == m::INT_32 || is_int16_or_lesser(b)
}

fn is_int64_or_lesser(b: u8) -> bool {
    b == m::INT_64 || is_int32_or_lesser(b)
}

fn is_string(b: u8) -> bool {
    is_tiny_string(b)
        || b == m::STRING_8
        || b == m::STRING_16
        || b == m::STRING_32
}

pub fn which(byte: u8) -> Option<&'static str> {
    match byte {
        m::NULL => Some("NULL"),
        m::TRUE => Some("TRUE"),
        m::FALSE => Some("FALSE"),
        _ if is_tiny_int(byte) => Some("TINY_INT"),
        m::INT_8 => Some("INT_8"),
        m::INT_16 => Some("INT_16"),
        m::INT_32 => Some("INT_32"),
        m::INT_64 => Some("INT_64"),
        m::FLOAT => Some("FLOAT"),
        _ if is_tiny_string(byte) => Some("TINY_STRING"),
        m::STRING_8 => Some("STRING_8"),
        m::STRING_16 => Some("STRING_16"),
        m::STRING_32 => Some("STRING_32"),
        _ if is_tiny_list(byte) => Some("TINY_LIST"),
        m::LIST_8 => Some("LIST_8"),
        m::LIST_16 => Some("LIST_16"),
        m::LIST_32 => Some("LIST_32"),
        _ if is_tiny_map(byte) => Some("TINY_MAP"),
        m::MAP_8 => Some("MAP_8"),
        m::MAP_16 => Some("MAP_16"),
        m::MAP_32 => Some("MAP_32"),
        _ if is_tiny_struct(byte) => Some("TINY_STRUCT"),
        m::STRUCT_8 => Some("STRUCT_8"),
        m::STRUCT_16 => Some("STRUCT_16"),
        _ => None
    }
}

#[derive(Debug)]
pub enum DecoderError {
    DecodingError(byteorder::Error),
    IoError(io::Error),
    WrongMarkerError(&'static str, String),
    WrongInputError(&'static str, String),
    InvalidUTF8Error,
    UnknownVariantError(String),
}

use self::DecoderError::*;

impl Error for DecoderError {
    fn description(&self) -> &str { "decoder error" }
}

impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WrongMarkerError(exp, ref got) | WrongInputError(exp, ref got) => {
                write!(f, "Expected '{}', Got '{}'", exp, got)
            }
            _ => fmt::Debug::fmt(&self, f)
        }
    }
}

impl From<byteorder::Error> for DecoderError {
    fn from(error: byteorder::Error) -> Self {
        DecoderError::DecodingError(error)
    }
}

impl From<io::Error> for DecoderError {
    fn from(error: io::Error) -> Self {
        DecoderError::IoError(error)
    }
}

impl From<string::FromUtf8Error> for DecoderError {
    fn from(_: string::FromUtf8Error) -> Self {
        DecoderError::InvalidUTF8Error
    }
}

macro_rules! wrong_marker {
    ($expected:expr, $got:ident) => {
        Err(WrongMarkerError(
            $expected,
            which($got)
                .map(|m| m.to_owned())
                .unwrap_or(format!("0x{:02X}", $got))
        ))
    }
}

macro_rules! wrong_input {
    ($expected:expr, $got:expr) => {
        Err(WrongInputError($expected, $got.to_owned()))
    }
}

pub struct PackstreamDecoder<'a, R: Read + 'a> {
    reader: BufReader<&'a mut R>,
}

impl<'a, R: Read> PackstreamDecoder<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        PackstreamDecoder {
            reader: BufReader::new(reader),
        }
    }
}

impl<'a, R: Read> Decoder for PackstreamDecoder<'a, R> {
    type Error = DecoderError;

    // Primitive types:
    fn read_nil(&mut self) -> Result<(), Self::Error> {
        let marker = try!(self.reader.read_u8());
        if marker != m::NULL {
            wrong_marker!("NULL", marker)
        } else {
            Ok(())
        }
    }

    #[cfg(target_pointer_width = "32")]
    fn read_usize(&mut self) -> Result<usize, Self::Error> {
        self.read_u32().map(|v| v as usize)
    }

    #[cfg(target_pointer_width = "64")]
    fn read_usize(&mut self) -> Result<usize, Self::Error> {
        self.read_u64().map(|v| v as usize)
    }

    fn read_u64(&mut self) -> Result<u64, Self::Error> {
        let value = try!(self.read_i64());

        if value < 0 {
            return wrong_input!("+INT_64", "-INTEGER")
        }

        Ok(value as u64)
    }

    fn read_u32(&mut self) -> Result<u32, Self::Error> {
        let value = try!(self.read_i32());

        if value < 0 {
            return wrong_input!("+INT_32", "-INTEGER")
        }

        Ok(value as u32)
    }

    fn read_u16(&mut self) -> Result<u16, Self::Error> {
        let value = try!(self.read_i16());

        if value < 0 {
            return wrong_input!("+INT_16", "-INTEGER")
        }

        Ok(value as u16)
    }

    fn read_u8(&mut self) -> Result<u8, Self::Error> {
        let value = try!(self.read_i8());

        if value < 0 {
            return wrong_input!("+INT_8", "-INTEGER")
        }

        Ok(value as u8)
    }

    fn read_isize(&mut self) -> Result<isize, Self::Error> {
        self.read_i64().map(|v| v as isize)
    }

    fn read_i64(&mut self) -> Result<i64, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int64_or_lesser(marker) {
            return wrong_marker!("INT_64", marker)
        }

        let value: i64;
        if is_tiny_int(marker) {
            value = marker as i64;
        } else if marker == m::INT_8 {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i64
        } else if marker == m::INT_16 {
            let value_read = try!(self.reader.read_i16::<BigEndian>());
            value = value_read as i64
        } else if marker == m::INT_32 {
            let value_read = try!(self.reader.read_i32::<BigEndian>());
            value = value_read as i64
        } else {
            let value_read = try!(self.reader.read_i64::<BigEndian>());
            value = value_read as i64
        }

        Ok(value)
    }

    fn read_i32(&mut self) -> Result<i32, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int32_or_lesser(marker) {
            return wrong_marker!("INT_32", marker)
        }

        let value: i32;
        if is_tiny_int(marker) {
            value = marker as i32;
        } else if marker == m::INT_8 {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i32
        } else if marker == m::INT_16 {
            let value_read = try!(self.reader.read_i16::<BigEndian>());
            value = value_read as i32
        } else {
            let value_read = try!(self.reader.read_i32::<BigEndian>());
            value = value_read as i32
        }

        Ok(value)
    }

    fn read_i16(&mut self) -> Result<i16, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int16_or_lesser(marker) {
            return wrong_marker!("INT_16", marker)
        }

        let value: i16;
        if is_tiny_int(marker) {
            value = marker as i16;
        } else if marker == m::INT_8 {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i16
        } else {
            let value_read = try!(self.reader.read_i16::<BigEndian>());
            value = value_read as i16
        }

        Ok(value)
    }

    fn read_i8(&mut self) -> Result<i8, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int8_or_lesser(marker) {
            return wrong_marker!("INT_8", marker)
        }

        let value: i8;
        if is_tiny_int(marker) {
            value = marker as i8;
        } else  {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i8
        }

        Ok(value)
    }

    fn read_bool(&mut self) -> Result<bool, Self::Error> {
        let marker = try!(self.reader.read_u8());
        match marker {
            m::TRUE => Ok(true),
            m::FALSE => Ok(false),
            _ => wrong_marker!("BOOLEAN", marker),
        }
    }

    fn read_f64(&mut self) -> Result<f64, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if marker != m::FLOAT {
            return wrong_marker!("FLOAT", marker)
        }

        self.reader.read_f64::<BigEndian>().map_err(From::from)
    }

    fn read_f32(&mut self) -> Result<f32, Self::Error> {
        self.read_f64().map(|v| v as f32)
    }

    fn read_char(&mut self) -> Result<char, Self::Error> {
        let value = try!(self.read_str());

        if value.len() > 1 { return wrong_input!("CHAR", "STRING") }

        Ok(value.char_at(0))
    }

    fn read_str(&mut self) -> Result<String, Self::Error> {
        let marker = try!(self.reader.read_u8());

        let size: usize;
        if is_tiny_string(marker) {
            size = (marker & 0b0000_1111) as usize;
        } else if marker == m::STRING_8 {
            size = try!(self.reader.read_u8()) as usize;
        } else if marker == m::STRING_16 {
            size = try!(self.reader.read_u16::<BigEndian>()) as usize;
        } else if marker == m::STRING_32 {
            size = try!(self.reader.read_u32::<BigEndian>()) as usize;
        } else {
            return wrong_marker!("STRING", marker)
        }

        let mut buf: Vec<u8> = Vec::with_capacity(size);
        try!(self.reader.read(&mut buf));

        String::from_utf8(buf).map_err(From::from)
    }

    // Compound types:
    fn read_enum<T, F>(&mut self, name: &str, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        f(self)
    }

    fn read_enum_variant<T, F>(&mut self, names: &[&str], f: F)
                               -> Result<T, Self::Error>
        where F: FnMut(&mut Self, usize) -> Result<T, Self::Error> {

        let marker = try!(self.reader.read_u8());
        let name: String;
        if is_string(marker) {
            let size: usize;
            if is_tiny_string(marker) {
                size = (marker & 0b0000_1111) as usize;
            } else if marker == m::STRING_8 {
                size = try!(self.reader.read_u8()) as usize;
            } else if marker == m::STRING_16 {
                size = try!(self.reader.read_u16::<BigEndian>()) as usize;
            } else if marker == m::STRING_32 {
                size = try!(self.reader.read_u32::<BigEndian>()) as usize;
            }

            let mut buf: Vec<u8> = Vec::with_capacity(size);
            try!(self.reader.read(&mut buf));

            name = try!(String::from_utf8(buf));
        } else if is_tiny_map(marker) {
            let size = 2;
            debug_assert!(size == marker & 0b0000_1111, "Invalid enum variant");
            name = try!(self.read_str());
        } else {
            return wrong_marker!("ENUM_VARIANT", marker)
        }

        let idx = match names.iter().position(|n| *n == name) {
            Some(idx) => idx,
            None => return Err(UnknownVariantError(name))
        };

        f(self, idx)
    }

    fn read_enum_variant_arg<T, F>(&mut self, a_idx: usize, f: F)
                                   -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        f(self)
    }

    fn read_enum_struct_variant<T, F>(&mut self, names: &[&str], f: F)
                                      -> Result<T, Self::Error>
        where F: FnMut(&mut Self, usize) -> Result<T, Self::Error> {

        self.read_enum_variant(names, f)
    }

    fn read_enum_struct_variant_field<T, F>(&mut self,
                                            f_name: &str,
                                            f_idx: usize,
                                            f: F)
                                            -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        self.read_enum_variant_arg(f_idx, f)
    }

    fn read_struct<T, F>(&mut self, s_name: &str, len: usize, f: F)
                         -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_struct_field<T, F>(&mut self,
                               f_name: &str,
                               f_idx: usize,
                               f: F)
                               -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_tuple<T, F>(&mut self, len: usize, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_tuple_arg<T, F>(&mut self, a_idx: usize, f: F)
                            -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_tuple_struct<T, F>(&mut self, s_name: &str, len: usize, f: F)
                               -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_tuple_struct_arg<T, F>(&mut self, a_idx: usize, f: F)
                                   -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    // Specialized types:
    fn read_option<T, F>(&mut self, f: F) -> Result<T, Self::Error>
        where F: FnMut(&mut Self, bool) -> Result<T, Self::Error> {

    }

    fn read_seq<T, F>(&mut self, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self, usize) -> Result<T, Self::Error> {

    }

    fn read_seq_elt<T, F>(&mut self, idx: usize, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_map<T, F>(&mut self, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self, usize) -> Result<T, Self::Error> {

    }

    fn read_map_elt_key<T, F>(&mut self, idx: usize, f: F)
                              -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    fn read_map_elt_val<T, F>(&mut self, idx: usize, f: F)
                              -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

    }

    // Failure
    fn error(&mut self, err: &str) -> Self::Error {

    }
}

use std::convert::From;
use std::error::Error;
use std::fmt;
use std::io::prelude::*;
use std::io;
use std::string;
use rustc_serialize::{Decodable, Decoder};
use byteorder::{self, ReadBytesExt, BigEndian};

use super::marker as m;

pub fn decode<T: Decodable, R: Read>(source: &mut R) -> DecodeResult<T> {
    let mut decoder = PackstreamDecoder::new(source);
    Decodable::decode(&mut decoder)
}

pub type DecodeResult<T> = Result<T, DecoderError>;

fn is_tiny_int_pos(b: u8) -> bool { b >> 7 == 0x00 }
fn is_tiny_int_neg(b: u8) -> bool { b >> 4 == m::TINY_INT_NEG_NIBBLE >> 4 }
fn is_tiny_int(b: u8) -> bool { is_tiny_int_pos(b) || is_tiny_int_neg(b) }

fn read_tiny_int(int: u8) -> i8 {
    if is_tiny_int_pos(int) { int as i8 }
    else { (int | 0b1111_0000) as i8 }
}

fn is_tiny_string(b: u8) -> bool { b >> 4 == m::TINY_STRING_NIBBLE >> 4 }
fn is_tiny_list(b: u8) -> bool { b >> 4 == m::TINY_LIST_NIBBLE >> 4 }
fn is_tiny_map(b: u8) -> bool { b >> 4 == m::TINY_MAP_NIBBLE >> 4 }
fn is_tiny_structure(b: u8) -> bool { b >> 4 == m::TINY_STRUCT_NIBBLE >> 4 }

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
    is_tiny_string(b) || b == m::STRING_8
        || b == m::STRING_16 || b == m::STRING_32
}

fn is_map(b: u8) -> bool {
    is_tiny_map(b) || b == m::MAP_8
        || b == m::MAP_16 || b == m::MAP_32
}

fn is_structure(b: u8) -> bool {
    is_tiny_structure(b) || b == m::STRUCT_8 || b == m::STRUCT_16
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
        _ if is_tiny_structure(byte) => Some("TINY_STRUCT"),
        m::STRUCT_8 => Some("STRUCT_8"),
        m::STRUCT_16 => Some("STRUCT_16"),
        _ => None
    }
}

#[derive(Debug)]
pub enum DecoderError {
    Io(io::Error),
    UnexpectedMarker(String, String),
    UnexpectedInput(String, String),
    UnknownVariant(String),
    WrongField(String, String),
    InvalidUTF8,
    ApplicationError(String),
    UnexpectedEOF,
}

use self::DecoderError as DecErr;

impl Error for DecoderError {
    fn description(&self) -> &str { "decoder error" }
}

impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecErr::UnexpectedMarker(ref exp, ref got) | DecErr::UnexpectedInput(ref exp, ref got) => {
                write!(f, "Expected '{}', Found '{}'", exp, got)
            }
            DecErr::WrongField(ref exp, ref got) => {
                write!(f, "Expected field '{}', Found '{}'", exp, got)
            }
            _ => fmt::Debug::fmt(&self, f)
        }
    }
}

impl From<byteorder::Error> for DecoderError {
    fn from(error: byteorder::Error) -> Self {
        match error {
            byteorder::Error::UnexpectedEOF => DecErr::UnexpectedEOF,
            byteorder::Error::Io(e) => DecErr::Io(e),
        }
    }
}

impl From<io::Error> for DecoderError {
    fn from(error: io::Error) -> Self {
        DecErr::Io(error)
    }
}

impl From<string::FromUtf8Error> for DecoderError {
    fn from(_: string::FromUtf8Error) -> Self {
        DecErr::InvalidUTF8
    }
}

macro_rules! wrong_marker {
    ($expected:expr, $got:ident) => {
        Err(DecErr::UnexpectedMarker(
            $expected,
            which($got)
                .map(|m| m.to_owned())
                .unwrap_or(format!("0x{:02X}", $got))
        ))
    }
}

macro_rules! wrong_input {
    ($expected:expr, $got:expr) => {
        Err(DecErr::UnexpectedInput($expected, $got))
    }
}

enum StructKind {
    Regular,
    Structure,
}

pub struct PackstreamDecoder<'a, R: Read + 'a> {
    reader: &'a mut R,
    struct_stack: Vec<StructKind>
}

impl<'a, R: Read> PackstreamDecoder<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        PackstreamDecoder {
            reader: reader,
            struct_stack: Vec::new(),
        }
    }
}

impl<'a, R: Read> Decoder for PackstreamDecoder<'a, R> {
    type Error = DecoderError;

    // Primitive types:
    fn read_nil(&mut self) -> Result<(), Self::Error> {
        let marker = try!(self.reader.read_u8());
        if marker != m::NULL {
            wrong_marker!("NULL".to_owned(), marker)
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
            return wrong_input!("+INT_64".to_owned(), "-INTEGER".to_owned())
        }

        Ok(value as u64)
    }

    fn read_u32(&mut self) -> Result<u32, Self::Error> {
        let value = try!(self.read_i32());

        if value < 0 {
            return wrong_input!("+INT_32".to_owned(), "-INTEGER".to_owned())
        }

        Ok(value as u32)
    }

    fn read_u16(&mut self) -> Result<u16, Self::Error> {
        let value = try!(self.read_i16());

        if value < 0 {
            return wrong_input!("+INT_16".to_owned(), "-INTEGER".to_owned())
        }

        Ok(value as u16)
    }

    fn read_u8(&mut self) -> Result<u8, Self::Error> {
        let value = try!(self.read_i8());

        if value < 0 {
            return wrong_input!("+INT_8".to_owned(), "-INTEGER".to_owned())
        }

        Ok(value as u8)
    }

    fn read_isize(&mut self) -> Result<isize, Self::Error> {
        self.read_i64().map(|v| v as isize)
    }

    fn read_i64(&mut self) -> Result<i64, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int64_or_lesser(marker) {
            return wrong_marker!("INT_64".to_owned(), marker)
        }

        let value: i64;
        if is_tiny_int(marker) {
            value = read_tiny_int(marker) as i64;
        } else if marker == m::INT_8 {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i64;
        } else if marker == m::INT_16 {
            let value_read = try!(self.reader.read_i16::<BigEndian>());
            value = value_read as i64;
        } else if marker == m::INT_32 {
            let value_read = try!(self.reader.read_i32::<BigEndian>());
            value = value_read as i64;
        } else {
            let value_read = try!(self.reader.read_i64::<BigEndian>());
            value = value_read as i64;
        }

        Ok(value)
    }

    fn read_i32(&mut self) -> Result<i32, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int32_or_lesser(marker) {
            return wrong_marker!("INT_32".to_owned(), marker)
        }

        let value: i32;
        if is_tiny_int(marker) {
            value = read_tiny_int(marker) as i32;
        } else if marker == m::INT_8 {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i32;
        } else if marker == m::INT_16 {
            let value_read = try!(self.reader.read_i16::<BigEndian>());
            value = value_read as i32;
        } else {
            let value_read = try!(self.reader.read_i32::<BigEndian>());
            value = value_read as i32;
        }

        Ok(value)
    }

    fn read_i16(&mut self) -> Result<i16, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if !is_int16_or_lesser(marker) {
            return wrong_marker!("INT_16".to_owned(), marker)
        }

        let value: i16;
        if is_tiny_int(marker) {
            value = read_tiny_int(marker) as i16;
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
            return wrong_marker!("INT_8".to_owned(), marker)
        }

        let value: i8;
        if is_tiny_int(marker) {
            value = read_tiny_int(marker);
        } else  {
            let value_read = try!(self.reader.read_i8());
            value = value_read
        }

        Ok(value)
    }

    fn read_bool(&mut self) -> Result<bool, Self::Error> {
        let marker = try!(self.reader.read_u8());
        match marker {
            m::TRUE => Ok(true),
            m::FALSE => Ok(false),
            _ => wrong_marker!("BOOLEAN".to_owned(), marker),
        }
    }

    fn read_f64(&mut self) -> Result<f64, Self::Error> {
        let marker = try!(self.reader.read_u8());
        if marker != m::FLOAT {
            return wrong_marker!("FLOAT".to_owned(), marker)
        }

        self.reader.read_f64::<BigEndian>().map_err(From::from)
    }

    fn read_f32(&mut self) -> Result<f32, Self::Error> {
        self.read_f64().map(|v| v as f32)
    }

    fn read_char(&mut self) -> Result<char, Self::Error> {
        let value = try!(self.read_str());

        if value.len() > 1 { return wrong_input!("CHAR".to_owned(), "STRING".to_owned()) }

        value.chars().nth(0).ok_or(
            DecErr::UnexpectedInput("CHAR".to_owned(), "Empty String".to_owned())
        )
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
            return wrong_marker!("STRING".to_owned(), marker)
        }

        let mut buf = [0u8; 4096];
        let mut store: Vec<u8> = Vec::with_capacity(size);

        let loops = (size as f32 / 4096.0).floor() as usize;
        for _ in 0..loops {
            let bytes = try!(self.reader.read(&mut buf));
            store.extend(buf[0..bytes].iter());
        }

        if size % 4096 > 0 {
            let mut buf = vec![0u8; size % 4096];
            try!(self.reader.read(&mut buf));
            store.append(&mut buf);
        }

        String::from_utf8(store).map_err(From::from)
    }

    // Compound types:
    fn read_enum<T, F>(&mut self, _: &str, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        f(self)
    }

    fn read_enum_variant<T, F>(&mut self, names: &[&str], mut f: F)
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
            } else {
                return wrong_marker!("STRING".to_owned(), marker)
            }

            let mut buf: Vec<u8> = Vec::with_capacity(size);
            try!(self.reader.read(&mut buf));

            name = try!(String::from_utf8(buf));
        } else if is_tiny_map(marker) {
            let size = 2;
            debug_assert!(size == marker & 0b0000_1111, "Invalid enum variant");
            name = try!(self.read_str());
        } else {
            return wrong_marker!("ENUM_VARIANT".to_owned(), marker)
        }

        let idx = match names.iter().position(|n| *n == name) {
            Some(idx) => idx,
            None => return Err(DecErr::UnknownVariant(name))
        };

        f(self, idx)
    }

    fn read_enum_variant_arg<T, F>(&mut self, _: usize, f: F)
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
                                            _: &str,
                                            f_idx: usize,
                                            f: F)
                                            -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        self.read_enum_variant_arg(f_idx, f)
    }

    fn read_struct<T, F>(&mut self, s_name: &str, len: usize, f: F)
                         -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        let marker = try!(self.reader.read_u8());

        let struct_kind: StructKind;
        let size: usize;
        if is_map(marker) {
            if is_tiny_map(marker) {
                size = (marker & 0b0000_1111) as usize;
            } else if marker == m::MAP_8 {
                size = try!(self.reader.read_u8()) as usize;
            } else if marker == m::MAP_16 {
                size = try!(self.reader.read_u16::<BigEndian>()) as usize;
            } else {
                size = try!(self.reader.read_u32::<BigEndian>()) as usize;
            }

            struct_kind = StructKind::Regular;
        } else if is_structure(marker) {
            if is_tiny_structure(marker) {
                size = (marker & 0b0000_1111) as usize;
            } else if marker == m::STRUCT_8 {
                size = try!(self.reader.read_u8()) as usize;
            } else {
                size = try!(self.reader.read_u16::<BigEndian>()) as usize;
            }

            struct_kind = StructKind::Structure;
        } else {
            return wrong_marker!("MAP or STRUCTURE".to_owned(), marker)
        }

        if size != len {
            return wrong_input!(format!("{} ({} fields)", s_name, len), format!("? ({} fields)", size))
        }

        self.struct_stack.push(struct_kind);
        let result = f(self);
        self.struct_stack.pop();
        result
    }

    fn read_struct_field<T, F>(&mut self,
                               f_name: &str,
                               _: usize,
                               f: F)
                               -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        match self.struct_stack.last() {
            Some(&StructKind::Regular) => {
                let prop = try!(self.read_str());
                if prop != f_name {
                    return Err(DecErr::WrongField(prop, f_name.to_owned()))
                }
            }
            Some(&StructKind::Structure) => {},
            _ => {}
        }

        f(self)
    }

    fn read_tuple<T, F>(&mut self, len: usize, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        self.read_seq(move |d, l| {
            if l == len {
                f(d)
            } else {
                wrong_input!(format!("Tuple{}", len), format!("Tuple{}", l))
            }
        })
    }

    fn read_tuple_arg<T, F>(&mut self, a_idx: usize, f: F)
                            -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        self.read_seq_elt(a_idx, f)
    }

    fn read_tuple_struct<T, F>(&mut self, _: &str, len: usize, f: F)
                               -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        self.read_tuple(len, f)
    }

    fn read_tuple_struct_arg<T, F>(&mut self, a_idx: usize, f: F)
                                   -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        self.read_tuple_arg(a_idx, f)
    }

    // Specialized types:
    fn read_option<T, F>(&mut self, mut f: F) -> Result<T, Self::Error>
        where F: FnMut(&mut Self, bool) -> Result<T, Self::Error> {

        let marker = try!(self.reader.read_u8());
        if marker == m::NULL { f(self, false) }
        else { f(self, true) }
    }

    fn read_seq<T, F>(&mut self, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self, usize) -> Result<T, Self::Error> {

        let marker = try!(self.reader.read_u8());

        let size: usize;
        if is_tiny_list(marker) {
            size = (marker & 0b0000_1111) as usize;
        } else if marker == m::LIST_8 {
            size = try!(self.reader.read_u8()) as usize;
        } else if marker == m::LIST_16 {
            size = try!(self.reader.read_u16::<BigEndian>()) as usize;
        } else if marker == m::LIST_32 {
            size = try!(self.reader.read_u32::<BigEndian>()) as usize;
        } else {
            return wrong_marker!("LIST".to_owned(), marker)
        }

        f(self, size)
    }

    fn read_seq_elt<T, F>(&mut self, _: usize, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        f(self)
    }

    fn read_map<T, F>(&mut self, f: F) -> Result<T, Self::Error>
        where F: FnOnce(&mut Self, usize) -> Result<T, Self::Error> {

        let marker = try!(self.reader.read_u8());

        let size: usize;
        if is_tiny_map(marker) {
            size = (marker & 0b0000_1111) as usize;
        } else if marker == m::MAP_8 {
            size = try!(self.reader.read_u8()) as usize;
        } else if marker == m::MAP_16 {
            size = try!(self.reader.read_u16::<BigEndian>()) as usize;
        } else if marker == m::MAP_32 {
            size = try!(self.reader.read_u32::<BigEndian>()) as usize;
        } else {
            return wrong_marker!("MAP".to_owned(), marker)
        }

        f(self, size)
    }

    fn read_map_elt_key<T, F>(&mut self, _: usize, f: F)
                              -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        f(self)
    }

    fn read_map_elt_val<T, F>(&mut self, _: usize, f: F)
                              -> Result<T, Self::Error>
        where F: FnOnce(&mut Self) -> Result<T, Self::Error> {

        f(self)
    }

    // Failure
    fn error(&mut self, err: &str) -> Self::Error {
        DecErr::ApplicationError(err.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::string::String;
    use std::io::Cursor;
    use super::decode;
    use ::v1::packstream::marker as m;

    #[test]
    fn deserialize_nil() {
        let mut input = Cursor::new(vec![0xC0]);
        let _: () = decode(&mut input).unwrap();

        let mut input = Cursor::new(vec![0xC0]);
        let result: Option<()> = decode(&mut input).unwrap();
        assert_eq!(None, result);
    }

    #[test]
    fn deserialize_bool() {
        let mut input = Cursor::new(vec![0xC3]);
        let result: bool = decode(&mut input).unwrap();
        assert_eq!(true, result);

        let mut input = Cursor::new(vec![0xC2]);
        let result: bool = decode(&mut input).unwrap();
        assert_eq!(false, result);
    }

    // Integer 64
    #[test]
    fn deserialize_int64_positive() {
        let mut input = Cursor::new(vec![m::INT_64, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let result: u64 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_POS_INT_64.1 as u64, result);
    }

    #[test]
    fn deserialize_int64_negative() {
        let mut input = Cursor::new(vec![m::INT_64, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let result: i64 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_NEG_INT_64.0, result);
    }

    #[test]
    fn deserialize_small_int_into_int64_positive() {
        let mut input = Cursor::new(vec![0x01]);
        let result: u64 = decode(&mut input).unwrap();
        assert_eq!(1, result);
    }

    #[test]
    fn deserialize_small_int_into_int64_negative() {
        let mut input = Cursor::new(vec![0xFF]);
        let result: i64 = decode(&mut input).unwrap();
        assert_eq!(-1, result);
    }

    #[test]
    #[should_panic(expected = "UnexpectedInput(\"+INT_64\", \"-INTEGER\")")]
    fn negative_int_into_u64_should_panic() {
        let mut input = Cursor::new(vec![0xFF]);
        let _: u64 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedMarker(\"INT_32\", \"INT_64\")")]
    fn positive_int64_into_smaller_should_fail() {
        let mut input = Cursor::new(vec![m::INT_64, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let _: u32 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedMarker(\"INT_32\", \"INT_64\")")]
    fn negative_int64_into_smaller_should_fail() {
        let mut input = Cursor::new(vec![m::INT_64, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let _: i32 = decode(&mut input).unwrap();
    }

    // Integer 32
    #[test]
    fn deserialize_int32_positive() {
        let mut input = Cursor::new(vec![m::INT_32, 0x7F, 0xFF, 0xFF, 0xFF]);
        let result: u32 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_POS_INT_32.1 as u32, result);
    }

    #[test]
    fn deserialize_int32_negative() {
        let mut input = Cursor::new(vec![m::INT_32, 0x80, 0x00, 0x00, 0x00]);
        let result: i32 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_NEG_INT_32.0 as i32, result);
    }

    #[test]
    fn deserialize_small_int_into_int32_positive() {
        let mut input = Cursor::new(vec![0x01]);
        let result: u32 = decode(&mut input).unwrap();
        assert_eq!(1, result);
    }

    #[test]
    fn deserialize_small_int_into_int32_negative() {
        let mut input = Cursor::new(vec![0xFF]);
        let result: i32 = decode(&mut input).unwrap();
        assert_eq!(-1, result);
    }

    #[test]
    #[should_panic(expected = "UnexpectedInput(\"+INT_32\", \"-INTEGER\")")]
    fn negative_int_into_u32_should_panic() {
        let mut input = Cursor::new(vec![0xFF]);
        let _: u32 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedMarker(\"INT_16\", \"INT_32\")")]
    fn positive_int32_into_smaller_should_fail() {
        let mut input = Cursor::new(vec![m::INT_32, 0x7F, 0xFF, 0xFF, 0xFF]);
        let _: u16 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedMarker(\"INT_16\", \"INT_32\")")]
    fn negative_int32_into_smaller_should_fail() {
        let mut input = Cursor::new(vec![m::INT_32, 0x80, 0x00, 0x00, 0x00]);
        let _: i16 = decode(&mut input).unwrap();
    }

    // Integer 16
    #[test]
    fn deserialize_int16_positive() {
        let mut input = Cursor::new(vec![m::INT_16, 0x7F, 0xFF]);
        let result: u16 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_POS_INT_16.1 as u16, result);
    }

    #[test]
    fn deserialize_int16_negative() {
        let mut input = Cursor::new(vec![m::INT_16, 0x80, 0x00]);
        let result: i16 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_NEG_INT_16.0 as i16, result);
    }

    #[test]
    fn deserialize_small_int_int16_positive() {
        let mut input = Cursor::new(vec![0x01]);
        let result: u16 = decode(&mut input).unwrap();
        assert_eq!(1, result);
    }

    #[test]
    fn deserialize_small_int_into_int16_negative() {
        let mut input = Cursor::new(vec![0xFF]);
        let result: i16 = decode(&mut input).unwrap();
        assert_eq!(-1, result);
    }

    #[test]
    #[should_panic(expected = "UnexpectedInput(\"+INT_16\", \"-INTEGER\")")]
    fn negative_int_into_u16_should_panic() {
        let mut input = Cursor::new(vec![0xFF]);
        let _: u16 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedMarker(\"INT_8\", \"INT_16\")")]
    fn positive_int16_into_smaller_should_fail() {
        let mut input = Cursor::new(vec![m::INT_16, 0x7F, 0xFF]);
        let _: u8 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedMarker(\"INT_8\", \"INT_16\")")]
    fn negative_int16_into_smaller_should_fail() {
        let mut input = Cursor::new(vec![m::INT_16, 0x80, 0x00]);
        let _: i8 = decode(&mut input).unwrap();
    }

    // Integer 8
    #[test]
    fn deserialize_int8_positive() {
        let mut input = Cursor::new(vec![0x7F]);
        let result: u8 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_TINY_INT.1 as u8, result);
    }

    #[test]
    fn deserialize_int8_negative() {
        let mut input = Cursor::new(vec![m::INT_8, 0x80]);
        let result: i8 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_NEG_INT_8.0 as i8, result);

        let mut input = Cursor::new(vec![0xF0]);
        let result: i8 = decode(&mut input).unwrap();
        assert_eq!(m::RANGE_TINY_INT.0 as i8, result);
    }

    #[test]
    #[should_panic(expected = "UnexpectedInput(\"+INT_8\", \"-INTEGER\")")]
    fn negative_int_into_u8_should_panic() {
        let mut input = Cursor::new(vec![m::INT_8, 0x80]);
        let _: u8 = decode(&mut input).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnexpectedInput(\"+INT_8\", \"-INTEGER\")")]
    fn negative_small_int_into_u8_should_panic() {
        let mut input = Cursor::new(vec![0xF0]);
        let _: u8 = decode(&mut input).unwrap();
    }

    #[test]
    fn deserialize_float_positive() {
        let mut input = Cursor::new(vec![m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]);
        let result: f64 = decode(&mut input).unwrap();
        assert_eq!(1.1, result);
    }

    #[test]
    fn deserialize_float_negative() {
        let mut input = Cursor::new(vec![m::FLOAT, 0xBF, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]);
        let result: f64 = decode(&mut input).unwrap();
        assert_eq!(-1.1, result);
    }

    #[test]
    fn deserialize_string32() {
        let size = 70_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRING_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, _| { acc.push(b'A'); acc }
        ));

        let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
        let result: String = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_string16() {
        let size = 5_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRING_16, 0x13, 0x88],
            |mut acc, _| { acc.push(b'A'); acc }
        ));

        let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
        let result: String = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_string8() {
        let size = 200;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRING_8, 0xC8],
            |mut acc, _| { acc.push(b'A'); acc }
        ));

        let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
        let result: String = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_tiny_string() {
        for marker in 0x80..0x8F {
            let size = marker - m::TINY_STRING_NIBBLE;
            let mut input = Cursor::new((0..size).fold(
                vec![marker],
                |mut acc, _| { acc.push(b'A'); acc }
            ));

            let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
            let result: String = decode(&mut input).unwrap();

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn deserialize_char() {
        for c in b'A'..b'Z' {
            let mut input = Cursor::new(vec![0x81, c]);
            let result: char = decode(&mut input).unwrap();

            assert_eq!(c as char, result);
        }
    }

    #[test]
    fn deserialize_list32() {
        let size = 70_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::LIST_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = vec![1; size];
        let result: Vec<u32> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_list16() {
        let size = 5_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::LIST_16, 0x13, 0x88],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = vec![1; size];
        let result: Vec<u32> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_list8() {
        let size = 200;
        let mut input = Cursor::new((0..size).fold(
            vec![m::LIST_8, 0xC8],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = vec![1; size];
        let result: Vec<u32> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_tiny_list() {
        for marker in 0x90..0x9F {
            let size = (marker - m::TINY_LIST_NIBBLE) as usize;
            let mut input = Cursor::new((0..size).fold(
                vec![marker],
                |mut acc, _| { acc.push(0x01); acc }
            ));

            let expected = vec![1; size];
            let result: Vec<u32> = decode(&mut input).unwrap();

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn deserialize_list_of_string() {
        let size = 3;

        let mut input = Cursor::new(vec![m::TINY_LIST_NIBBLE + size as u8,
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
                                    0x77, 0x78, 0x79, 0x7A]);

        let result: Vec<String> = decode(&mut input).unwrap();
        let expected = vec!["abcdefghijklmnopqrstuvwxyz"; size];
        assert_eq!(expected, result);
    }
}

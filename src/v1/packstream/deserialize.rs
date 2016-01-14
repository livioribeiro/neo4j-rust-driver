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

    fn read_string_data(&mut self, marker: u8) -> Result<String, DecoderError> {
        let size;
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

        let mut store;
        if size < 4096 {
            store = vec![0u8; size];
            try!(self.reader.read(&mut store));
        } else {
            store = Vec::with_capacity(size);
            let mut buf = [0u8; 4096];

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
        }

        String::from_utf8(store).map_err(From::from)
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

        if !is_string(marker) {
            return wrong_marker!("STRING".to_owned(), marker)
        }

        self.read_string_data(marker)
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
            name = try!(self.read_string_data(marker));
        } else if is_tiny_map(marker) {
            let size = 1;
            if size != marker & 0b0000_1111 {
                return wrong_input!("Map(1)".to_owned(), format!("Map({})", marker & 0b0000_1111))
            }
            name = try!(self.read_str());
            try!(self.read_seq(|_, _| Ok(())));
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

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
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
                 0x77, 0x78, 0x79, 0x7A]
        );

        let result: Vec<String> = decode(&mut input).unwrap();
        let expected = vec!["abcdefghijklmnopqrstuvwxyz"; size];

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_list_of_int() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 m::INT_16, 0x7D, 0x00,
                 m::INT_16, 0x7D, 0x00,
                 m::INT_16, 0x7D, 0x00]
             );

        let result: Vec<u32> = decode(&mut input).unwrap();
        let expected = vec![32_000; size];

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_list_of_float() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]
             );

        let result: Vec<f32> = decode(&mut input).unwrap();
        let expected = vec![1.1; size];

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_list_of_bool() {
        let size = 4;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 m::TRUE, m::FALSE, m::TRUE, m::FALSE]
             );

        let result: Vec<bool> = decode(&mut input).unwrap();
        let expected = vec![true, false, true, false];

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_tuple() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 0x01,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 m::TINY_STRING_NIBBLE + 1, 0x41]
             );

        let result: (u32, f64, String) = decode(&mut input).unwrap();
        let expected = (1, 1.1, "A".to_owned());

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map32() {
        let size = 70_000;

        let mut input = Cursor::new((0..size).fold(
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
        ));

        let expected = (0..size).fold(
            BTreeMap::<String, u16>::new(),
            |mut acc, i| { acc.insert(format!("{:05}", i), 1); acc }
        );

        let result: BTreeMap<String, u16> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map16() {
        let size = 5_000;

        let mut input = Cursor::new((0..size).fold(
            vec![m::MAP_16, 0x13, 0x88],
            |mut acc, i| {
                let b1 = 48 + ((i % 10000) / 1000) as u8;
                let b2 = 48 + ((i % 1000) / 100) as u8;
                let b3 = 48 + ((i % 100) / 10) as u8;
                let b4 = 48 + (i % 10) as u8;
                acc.extend([0x84, b1, b2, b3, b4, 0x01].iter());
                acc
            }
        ));

        let expected = (0..size).fold(
            BTreeMap::<String, u16>::new(),
            |mut acc, i| { acc.insert(format!("{:04}", i), 1); acc }
        );

        let result: BTreeMap<String, u16> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map8() {
        let size = 200;

        let mut input = Cursor::new((0..size).fold(
            vec![m::MAP_8, 0xC8],
            |mut acc, i| {
                let b1 = 48 + ((i % 1000) / 100) as u8;
                let b2 = 48 + ((i % 100) / 10) as u8;
                let b3 = 48 + (i % 10) as u8;
                acc.extend([0x83, b1, b2, b3, 0x01].iter());
                acc
            }
        ));

        let expected = (0..size).fold(
            BTreeMap::<String, u16>::new(),
            |mut acc, i| { acc.insert(format!("{:03}", i), 1); acc }
        );

        let result: BTreeMap<String, u16> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_tiny_map() {
        let size = 3;

        let mut input = Cursor::new((0..size).fold(
            vec![m::TINY_MAP_NIBBLE + size],
            |mut acc, i| {
                acc.extend([0x81, 0x30 + i].iter());
                acc.push(0x01);
                acc
            }
        ));

        let expected = (0..size).fold(
            BTreeMap::<String, u16>::new(),
            |mut acc, i| { acc.insert(format!("{}", i), 1); acc }
        );

        let result: BTreeMap<String, u16> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map_of_string() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
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
                 0x77, 0x78, 0x79, 0x7A]
        );

        let expected = {
            let mut expected: BTreeMap<String, String> = BTreeMap::new();
            expected.insert("A".to_owned(), "abcdefghijklmnopqrstuvwxyz".to_owned());
            expected.insert("B".to_owned(), "abcdefghijklmnopqrstuvwxyz".to_owned());
            expected.insert("C".to_owned(), "abcdefghijklmnopqrstuvwxyz".to_owned());
            expected
        };

        let result: BTreeMap<String, String> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map_of_int() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
                 0x81, 0x41, m::INT_16, 0x7D, 0x00,
                 0x81, 0x42, m::INT_16, 0x7D, 0x00,
                 0x81, 0x43, m::INT_16, 0x7D, 0x00]
        );

        let expected = {
            let mut expected: BTreeMap<String, u32> = BTreeMap::new();
            expected.insert("A".to_owned(), 32_000);
            expected.insert("B".to_owned(), 32_000);
            expected.insert("C".to_owned(), 32_000);
            expected
        };

        let result: BTreeMap<String, u32> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map_of_float() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
                 0x81, 0x41, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 0x81, 0x42, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 0x81, 0x43, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]
        );

        let expected = {
            let mut expected: BTreeMap<String, f64> = BTreeMap::new();
            expected.insert("A".to_owned(), 1.1);
            expected.insert("B".to_owned(), 1.1);
            expected.insert("C".to_owned(), 1.1);
            expected
        };

        let result: BTreeMap<String, f64> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_map_of_bool() {
        let size = 4;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
                 0x81, 0x41, m::TRUE,
                 0x81, 0x42, m::FALSE,
                 0x81, 0x43, m::TRUE,
                 0x81, 0x44, m::FALSE]
        );

        let expected = {
            let mut expected: BTreeMap<String, bool> = BTreeMap::new();
            expected.insert("A".to_owned(), true);
            expected.insert("B".to_owned(), false);
            expected.insert("C".to_owned(), true);
            expected.insert("D".to_owned(), false);
            expected
        };

        let result: BTreeMap<String, bool> = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_struct16() {
        #[derive(RustcDecodable, Debug, PartialEq)]
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

        let mut input = Cursor::new(vec![m::MAP_16, 0x01, 0x00,
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
        ]);

        let expected = MyStruct {
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

        let result: MyStruct = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_struct8() {
        #[derive(RustcDecodable, Debug, PartialEq)]
        #[allow(non_snake_case)]
        struct MyStruct {
            A: u16, B: u16, C: u16, D: u16,
            E: u16, F: u16, G: u16, H: u16,
            I: u16, J: u16, K: u16, L: u16,
            M: u16, N: u16, O: u16, P: u16,
        }

        let mut input = Cursor::new(vec![m::MAP_8, 0x10,
            0x81, 0x41, 0x01, 0x81, 0x42, 0x01, 0x81, 0x43, 0x01, 0x81, 0x44, 0x01,
            0x81, 0x45, 0x01, 0x81, 0x46, 0x01, 0x81, 0x47, 0x01, 0x81, 0x48, 0x01,
            0x81, 0x49, 0x01, 0x81, 0x4A, 0x01, 0x81, 0x4B, 0x01, 0x81, 0x4C, 0x01,
            0x81, 0x4D, 0x01, 0x81, 0x4E, 0x01, 0x81, 0x4F, 0x01, 0x81, 0x50, 0x01
        ]);

        let expected = MyStruct {
            A: 1, B: 1, C: 1, D: 1,
            E: 1, F: 1, G: 1, H: 1,
            I: 1, J: 1, K: 1, L: 1,
            M: 1, N: 1, O: 1, P: 1,
        };

        let result: MyStruct = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_tiny_struct() {
        #[derive(RustcDecodable, Debug, PartialEq)]
        #[allow(non_snake_case)]
        struct MyStruct {
            A: u32,
            B: f64,
            C: String,
        }

        let mut input = Cursor::new(vec![m::TINY_MAP_NIBBLE + 0x03,
            0x81, 0x41, 0x01,
            0x81, 0x42, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
            0x81, 0x43, 0x81, 0x43
        ]);

        let expected = MyStruct {
            A: 1,
            B: 1.1,
            C: "C".to_owned(),
        };

        let result: MyStruct = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_structure() {
        #[derive(RustcDecodable, Debug, PartialEq)]
        #[allow(non_snake_case)]
        struct MyStruct {
            A: u32,
            B: f64,
            C: String,
        }

        let mut input = Cursor::new(vec![m::TINY_STRUCT_NIBBLE + 0x03,
            0x01,
            m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
            0x81, 0x43
        ]);

        let expected = MyStruct {
            A: 1,
            B: 1.1,
            C: "C".to_owned(),
        };

        let result: MyStruct = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_enum() {
        #[derive(RustcDecodable, Debug, PartialEq)]
        enum MyEnum {
            A, B,
        }

        let mut input = Cursor::new(vec![0x81, 0x41]);

        let expected = MyEnum::A;
        let result: MyEnum = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn deserialize_enum_tuple_variant() {
        #[derive(RustcDecodable, Debug, PartialEq)]
        enum MyEnum {
            A(u16, u16), B(f32, f32),
        }

        let mut input = Cursor::new(vec![m::TINY_MAP_NIBBLE + 0x01,
                                         0x81, 0x41,
                                         0x92, 0x01, 0x02]);

        let expected = MyEnum::A(1, 2);
        let result: MyEnum = decode(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    #[should_panic(expected = "UnexpectedInput(\"Map(1)\", \"Map(2)\")")]
    fn enum_tuple_variant_with_wrong_map_size_should_fail() {
        #[derive(RustcDecodable, Debug, PartialEq)]
        enum MyEnum {
            A(u16, u16), B(f32, f32),
        }

        let mut input = Cursor::new(vec![m::TINY_MAP_NIBBLE + 0x02,
                                         0x81, 0x41,
                                         0x92, 0x01, 0x02]);

        let expected = MyEnum::A(1, 2);
        let _: MyEnum = decode(&mut input).unwrap();
    }
}

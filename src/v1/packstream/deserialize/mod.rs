use std::collections::VecDeque;
use std::io::Read;

use serde::de::{self, Error as SerdeError};
use byteorder::{ReadBytesExt, BigEndian};

pub mod error;
mod visitor;
mod types;

#[cfg(test)]
mod tests;

use super::marker as M;
use self::visitor::{SeqVisitor, MapVisitor};

pub use self::error::DeserializerError;
use self::DeserializerError as DesErr;

pub fn from_reader<T: de::Deserialize, R: Read>(source: &mut R) -> DeserializeResult<T> {
    let mut decoder = Deserializer::new(source);
    de::Deserialize::deserialize(&mut decoder)
}

pub type DeserializeResult<T> = Result<T, DeserializerError>;

macro_rules! wrong_marker {
    ($expected:expr, $got:ident) => {
        Err(DesErr::UnexpectedMarker(
            $expected,
            types::which($got)
                .map(|m| m.to_owned())
                .unwrap_or(format!("0x{:02X}", $got))
        ))
    }
}

macro_rules! wrong_input {
    ($expected:expr, $got:expr) => {
        Err(DesErr::UnexpectedInput($expected, $got))
    }
}

pub struct Deserializer<R> {
    reader: R,
    buffer: VecDeque<u8>,
}

impl<R: Read> Deserializer<R>
{
    pub fn new(reader: R) -> Self {
        Deserializer {
            reader: reader,
            buffer: VecDeque::new(),
        }
    }

    fn peek(&mut self) -> Result<u8, DesErr> {
        match self.buffer.front().map(|b| *b) {
            Some(byte) => Ok(byte),
            None => self.peek_next()
        }
    }

    fn bump(&mut self) {
        self.buffer.pop_front();
    }

    fn peek_next(&mut self) -> Result<u8, DesErr> {
        self.reader.read_u8()
            .map(|b| { self.buffer.push_back(b); b })
            .map_err(From::from)
    }

    fn next(&mut self) -> Result<u8, DesErr> {
        if let Some(b) = self.buffer.pop_front() {
            Ok(b)
        } else {
            self.reader.read_u8().map_err(From::from)
        }
    }

    fn parse_value<V>(&mut self, mut visitor: V) -> Result<V::Value, DesErr>
        where V: de::Visitor,
    {
        let result = match try!(self.peek()) {
            0xC0 =>
                visitor.visit_unit(),
            0xC3 =>
                visitor.visit_bool(true),
            0xC2 =>
                visitor.visit_bool(false),
            0x00...0x7F | 0xF0...0xFF | 0xC8 =>
                self.parse_int().and_then(|v| if v > 0 {visitor.visit_u8(v as u8)} else {visitor.visit_i8(v as i8)}),
            0xC9 =>
                self.parse_int().and_then(|v| if v > 0 {visitor.visit_u16(v as u16)} else {visitor.visit_i16(v as i16)}),
            0xCA =>
                self.parse_int().and_then(|v| if v > 0 {visitor.visit_u32(v as u32)} else {visitor.visit_i32(v as i32)}),
            0xCB =>
                self.parse_int().and_then(|v| if v > 0 {visitor.visit_u64(v as u64)} else {visitor.visit_i64(v)}),
            0xC1 =>
                self.parse_float().and_then(|v| visitor.visit_f64(v)),
            0x80...0x8F | 0xD0 | 0xD1 | 0xD2 =>
                self.parse_string().and_then(|v| visitor.visit_str(v.as_ref())),
            0x90...0x9F | 0xD4 | 0xD5 | 0xD6 => {
                let marker = try!(self.next());

                let size = match marker {
                    0x90...0x9F => (marker & 0b0000_1111) as usize,
                    0xD4 => try!(self.reader.read_u8()) as usize,
                    0xD5 => try!(self.reader.read_u16::<BigEndian>()) as usize,
                    0xD6 => try!(self.reader.read_u32::<BigEndian>()) as usize,
                    _ => unreachable!()
                };

                let seq_visitor = SeqVisitor::new(self, size);
                visitor.visit_seq(seq_visitor)
            },
            0xA0...0xAF | 0xD8 | 0xD9 | 0xDA => {
                let marker = try!(self.next());

                let size = match marker {
                    0xA0...0xAF => (marker & 0b0000_1111) as usize,
                    0xD8 => try!(self.reader.read_u8()) as usize,
                    0xD9 => try!(self.reader.read_u16::<BigEndian>()) as usize,
                    0xDA => try!(self.reader.read_u32::<BigEndian>()) as usize,
                    _ => unreachable!()
                };

                let map_visitor = MapVisitor::new(self, size);
                visitor.visit_map(map_visitor)
            },
            0xB0...0xBF | 0xDC | 0xDD => {
                let marker = try!(self.next());

                let size = match marker {
                    0xB0...0xBF => (marker & 0b0000_1111) as usize,
                    0xDC => try!(self.reader.read_u8()) as usize,
                    0xDD => try!(self.reader.read_u16::<BigEndian>()) as usize,
                    _ => unreachable!()
                };

                let seq_visitor = SeqVisitor::new(self, size + 1);
                visitor.visit_seq(seq_visitor)
            }
            value => return Err(DesErr::UnexpectedMarker("Type Marker".to_owned(), format!("{:02X}", value)))
        };

        self.bump();

        result
    }

    fn parse_int(&mut self) -> Result<i64, DesErr> {
        let marker = try!(self.next());

        if !types::is_int64_or_lesser(marker) {
            return Err(DesErr::UnexpectedType("Integer"))
        }

        let value: i64;
        if types::is_tiny_int(marker) {
            value = types::read_tiny_int(marker) as i64;
        } else if marker == M::INT_8 {
            let value_read = try!(self.reader.read_i8());
            value = value_read as i64;
        } else if marker == M::INT_16 {
            let value_read = try!(self.reader.read_i16::<BigEndian>());
            value = value_read as i64;
        } else if marker == M::INT_32 {
            let value_read = try!(self.reader.read_i32::<BigEndian>());
            value = value_read as i64;
        } else {
            let value_read = try!(self.reader.read_i64::<BigEndian>());
            value = value_read as i64;
        }

        Ok(value)
    }

    fn parse_float(&mut self) -> Result<f64, DesErr> {
        let marker = try!(self.next());

        if marker != M::FLOAT {
            return wrong_marker!("FLOAT".to_owned(), marker)
        }

        self.reader.read_f64::<BigEndian>().map_err(From::from)
    }

    fn parse_string(&mut self) -> Result<String, DesErr> {
        let marker = try!(self.next());

        let size;
        if types::is_tiny_string(marker) {
            size = (marker & 0b0000_1111) as usize;
        } else if marker == M::STRING_8 {
            size = try!(self.reader.read_u8()) as usize;
        } else if marker == M::STRING_16 {
            size = try!(self.reader.read_u16::<BigEndian>()) as usize;
        } else if marker == M::STRING_32 {
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

impl<R: Read> de::Deserializer for Deserializer<R> {
    type Error = DesErr;

    fn deserialize<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.parse_value(visitor)
    }

    fn deserialize_bool<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_usize<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        let value = try!(self.parse_int());
        if value < 0 { return Err(DesErr::invalid_type(de::Type::Isize)) }
        visitor.visit_usize(value as usize)
    }

    fn deserialize_u8<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        let value = try!(self.parse_int());
        if value < 0 {
            if value < ::std::i32::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I64))
            } else if value < ::std::i16::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I32))
            } else if value < ::std::i8::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I16))
            } else {
                return Err(DesErr::invalid_type(de::Type::I8))
            }
        } else {
            if value > ::std::u32::MAX as i64 {
                return Err(DesErr::invalid_type(de::Type::U64))
            } else if value > ::std::u16::MAX as i64 {
                return Err(DesErr::invalid_type(de::Type::U32))
            } else if value > ::std::u8::MAX as i64 {
                return Err(DesErr::invalid_type(de::Type::U16))
            }
        }
        visitor.visit_u8(value as u8)
    }

    fn deserialize_u16<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        let value = try!(self.parse_int());
        if value < 0 {
            if value < ::std::i32::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I64))
            } else if value < ::std::i16::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I32))
            } else if value < ::std::i8::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I16))
            } else {
                return Err(DesErr::invalid_type(de::Type::I8))
            }
        } else {
            if value > ::std::u32::MAX as i64 {
                return Err(DesErr::invalid_type(de::Type::U64))
            } else if value > ::std::u16::MAX as i64 {
                return Err(DesErr::invalid_type(de::Type::U32))
            }
        }
        visitor.visit_u16(value as u16)
    }

    fn deserialize_u32<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        let value = try!(self.parse_int());
        if value < 0 {
            if value < ::std::i32::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I64))
            } else if value < ::std::i16::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I32))
            } else if value < ::std::i8::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I16))
            } else {
                return Err(DesErr::invalid_type(de::Type::I8))
            }
        } else {
            if value > ::std::u32::MAX as i64 {
                return Err(DesErr::invalid_type(de::Type::U64))
            }
        }
        visitor.visit_u32(value as u32)
    }

    fn deserialize_u64<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        let value = try!(self.parse_int());
        if value < 0 {
            if value < ::std::i32::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I64))
            } else if value < ::std::i16::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I32))
            } else if value < ::std::i8::MIN as i64 {
                return Err(DesErr::invalid_type(de::Type::I16))
            } else {
                return Err(DesErr::invalid_type(de::Type::I8))
            }
        }
        visitor.visit_u64(value as u64)
    }

    fn deserialize_isize<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_i8<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_i16<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_i32<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_i64<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_f32<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_f64<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_char<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_str<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_string<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_unit<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_option<V>(&mut self, mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        match try!(self.peek()) {
            M::NULL => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_seq<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_seq_fixed_size<V>(&mut self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_bytes<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_map<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_unit_struct<V>(&mut self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_newtype_struct<V>(&mut self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_tuple_struct<V>(&mut self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_struct<V>(&mut self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_struct_field<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_tuple<V>(&mut self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }

    fn deserialize_enum<V>(
        &mut self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V
    ) -> Result<V::Value, Self::Error>
        where V: de::EnumVisitor
    {
        Err(DesErr::invalid_type(de::Type::Enum))
    }

    fn deserialize_ignored_any<V>(&mut self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor
    {
        self.deserialize(visitor)
    }
}

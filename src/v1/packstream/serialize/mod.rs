use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

use byteorder::{WriteBytesExt, BigEndian};
use serde::ser::{self, Serialize};

use super::marker as M;
use super::STRUCTURE_IDENTIFIER;

#[cfg(test)]
mod tests;

pub fn serialize<T: ser::Serialize>(value: &T) -> Result<Vec<u8>, SerializerError> {
    let mut buf = io::Cursor::new(Vec::new());
    {
        let mut encoder = Serializer::new(&mut buf);
        try!(value.serialize(&mut encoder));
    }
    Ok(buf.into_inner())
}

#[derive(Debug)]
pub enum SerializerError {
    IoError(io::Error),
    InvalidStructureLength,
    Custom(String),
}

impl Error for SerializerError {
    fn description(&self) -> &str { "serializer error" }
}

impl ser::Error for SerializerError {
    fn custom<T: Into<String>>(msg: T) -> Self {
        SerializerError::Custom(msg.into())
    }
}

impl Display for SerializerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl From<io::Error> for SerializerError {
    fn from(error: io::Error) -> Self {
        SerializerError::IoError(error)
    }
}

/// A structure for serializing Rust values into JSON.
pub struct Serializer<W> {
    writer: W,
}

impl<W> Serializer<W>
    where W: io::Write,
{
    /// Creates a new PackStream serializer.
    #[inline]
    pub fn new(writer: W) -> Self {
        Serializer {
            writer: writer,
        }
    }

    /// Unwrap the `Writer` from the `Serializer`.
    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }
}

#[doc(hidden)]
#[derive(Eq, PartialEq)]
pub enum StructState {
    Map,
    Structure,
}

impl<W> ser::Serializer for Serializer<W>
    where W: io::Write,
{
    type Error = SerializerError;

    type SeqState = ();
    type TupleState = ();
    type TupleStructState = ();
    type TupleVariantState = ();
    type MapState = ();
    type StructState = StructState;
    type StructVariantState = StructState;

    #[inline]
    fn serialize_bool(&mut self, value: bool) -> Result<(), Self::Error> {
        if value {
            self.writer.write_u8(M::TRUE).map_err(From::from)
        } else {
            self.writer.write_u8(M::FALSE).map_err(From::from)
        }
    }

    #[inline]
    fn serialize_isize(&mut self, value: isize) -> Result<(), Self::Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i8(&mut self, value: i8) -> Result<(), Self::Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i16(&mut self, value: i16) -> Result<(), Self::Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i32(&mut self, value: i32) -> Result<(), Self::Error> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i64(&mut self, value: i64) -> Result<(), Self::Error> {
        if (value >= M::RANGE_POS_INT_64.0 && value <= M::RANGE_POS_INT_64.1)
            || (value >= M::RANGE_NEG_INT_64.0 && value <= M::RANGE_NEG_INT_64.1)
        {
            try!(self.writer.write_u8(M::INT_64));
            try!(self.writer.write_i64::<BigEndian>(value));
        } else if (value >= M::RANGE_POS_INT_32.0 && value <= M::RANGE_POS_INT_32.1)
            || (value >= M::RANGE_NEG_INT_32.0 && value <= M::RANGE_NEG_INT_32.1)
        {
            try!(self.writer.write_u8(M::INT_32));
            try!(self.writer.write_i32::<BigEndian>(value as i32));
        } else if (value >= M::RANGE_POS_INT_16.0 && value <= M::RANGE_POS_INT_16.1)
            || (value >= M::RANGE_NEG_INT_16.0 && value <= M::RANGE_NEG_INT_16.1)
        {
            try!(self.writer.write_u8(M::INT_16));
            try!(self.writer.write_i16::<BigEndian>(value as i16));
        } else if value >= M::RANGE_TINY_INT.0 && value <= M::RANGE_TINY_INT.1  {
            try!(self.writer.write_i8(value as i8));
        } else if value >= M::RANGE_NEG_INT_8.0 && value <= M::RANGE_NEG_INT_8.1 {
            try!(self.writer.write_u8(M::INT_8));
            try!(self.writer.write_i8(value as i8));
        }

        Ok(())
    }

    #[inline]
    fn serialize_usize(&mut self, value: usize) -> Result<(), Self::Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u8(&mut self, value: u8) -> Result<(), Self::Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u16(&mut self, value: u16) -> Result<(), Self::Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u32(&mut self, value: u32) -> Result<(), Self::Error> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error> {
        if value >= M::RANGE_POS_INT_64.0 as u64 && value <= M::RANGE_POS_INT_64.1 as u64 {
            try!(self.writer.write_u8(M::INT_64));
            try!(self.writer.write_u64::<BigEndian>(value));
        } else if value >= M::RANGE_POS_INT_32.0 as u64 && value <= M::RANGE_POS_INT_32.1 as u64 {
            try!(self.writer.write_u8(M::INT_32));
            try!(self.writer.write_u32::<BigEndian>(value as u32));
        } else if value >= M::RANGE_POS_INT_16.0 as u64 && value <= M::RANGE_POS_INT_16.1 as u64 {
            try!(self.writer.write_u8(M::INT_16));
            try!(self.writer.write_u16::<BigEndian>(value as u16));
        } else if value <= M::RANGE_TINY_INT.1 as u64 {
            try!(self.writer.write_u8(value as u8));
        }

        Ok(())
    }

    #[inline]
    fn serialize_f32(&mut self, value: f32) -> Result<(), Self::Error> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(&mut self, value: f64) -> Result<(), Self::Error> {
        try!(self.writer.write_u8(M::FLOAT));
        self.writer.write_f64::<BigEndian>(value).map_err(From::from)
    }

    #[inline]
    fn serialize_char(&mut self, value: char) -> Result<(), Self::Error> {
        let mut string_value = String::new();
        string_value.push(value);
        self.serialize_str(&string_value)
    }

    #[inline]
    fn serialize_str(&mut self, value: &str) -> Result<(), Self::Error> {
        let bytes = value.as_bytes();
        let size = bytes.len();

        if size <= M::USE_TINY_STRING {
            try!(self.writer.write_u8(M::TINY_STRING_NIBBLE | size as u8));
        } else if size <= M::USE_STRING_8 {
            try!(self.writer.write_u8(M::STRING_8));
            try!(self.writer.write_u8(size as u8));
        } else if size <= M::USE_STRING_16 {
            try!(self.writer.write_u8(M::STRING_16));
            try!(self.writer.write_u16::<BigEndian>(size as u16));
        } else if size <= M::USE_STRING_32 {
            try!(self.writer.write_u8(M::STRING_32));
            try!(self.writer.write_u32::<BigEndian>(size as u32));
        }

        try!(self.writer.write_all(bytes));

        Ok(())
    }

    #[inline]
    fn serialize_bytes(&mut self, value: &[u8]) -> Result<(), Self::Error> {
        let mut state = try!(self.serialize_seq(Some(value.len())));
        for byte in value {
            try!(self.serialize_seq_elt(&mut state, byte));
        }
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_unit(&mut self) -> Result<(), Self::Error> {
        self.writer.write_u8(M::NULL).map_err(From::from)
    }

    #[inline]
    fn serialize_unit_struct(&mut self, _name: &'static str) -> Result<(), Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str
    ) -> Result<(), Self::Error> {
        self.serialize_str(variant)
    }

    /// Serialize newtypes without an object wrapper.
    #[inline]
    fn serialize_newtype_struct<T>(
        &mut self,
        _name: &'static str,
        value: T
    ) -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        value: T
    ) -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        try!(self.writer.write_u8(M::TINY_MAP_NIBBLE | 0x01));
        try!(self.serialize_str(variant));
        value.serialize(self)
    }

    #[inline]
    fn serialize_none(&mut self) -> Result<(), Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T>(&mut self, value: T) -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(&mut self, len: Option<usize>) -> Result<Self::SeqState, Self::Error> {
        match len {
            Some(0) | None => {
                try!(self.writer.write_u8(M::TINY_LIST_NIBBLE | 0x00));
            }
            Some(len) => {
                if len <= M::USE_TINY_LIST as usize {
                    try!(self.writer.write_u8(M::TINY_LIST_NIBBLE | len as u8));
                } else if len <= M::USE_LIST_8 as usize {
                    try!(self.writer.write_u8(M::LIST_8));
                    try!(self.writer.write_u8(len as u8));
                } else if len <= M::USE_LIST_16 as usize {
                    try!(self.writer.write_u8(M::LIST_16));
                    try!(self.writer.write_u16::<BigEndian>(len as u16));
                } else if len <= M::USE_LIST_32 as usize {
                    try!(self.writer.write_u8(M::LIST_32));
                    try!(self.writer.write_u32::<BigEndian>(len as u32));
                }
            }
        }

        Ok(())
    }

    #[inline]
    fn serialize_seq_elt<T: ser::Serialize>(
        &mut self,
        _state: &mut Self::SeqState,
        value: T
    ) -> Result<(), Self::Error>
        where T: ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq_end(&mut self, _state: Self::SeqState) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_seq_fixed_size(&mut self, size: usize) -> Result<Self::SeqState, Self::Error> {
        self.serialize_seq(Some(size))
    }

    #[inline]
    fn serialize_tuple(&mut self, len: usize) -> Result<Self::TupleState, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_elt<T: ser::Serialize>(
        &mut self,
        state: &mut Self::TupleState,
        value: T
    ) -> Result<(), Self::Error> {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_end(&mut self, state: Self::TupleState) -> Result<(), Self::Error> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_tuple_struct(
        &mut self,
        _name: &'static str,
        len: usize
    ) -> Result<Self::TupleStructState, Self::Error> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct_elt<T: ser::Serialize>(
        &mut self,
        state: &mut Self::TupleStructState,
        value: T
    ) -> Result<(), Self::Error> {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_struct_end(
        &mut self, state: Self::TupleStructState
    ) -> Result<(), Self::Error> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_tuple_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        len: usize
    ) -> Result<Self::TupleVariantState, Self::Error> {
        try!(self.serialize_map(Some(1)));
        try!(self.serialize_str(variant));
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant_elt<T: ser::Serialize>(
        &mut self,
        state: &mut Self::TupleVariantState,
        value: T
    ) -> Result<(), Self::Error> {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_variant_end(&mut self, state: Self::TupleVariantState) -> Result<(), Self::Error> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_map(&mut self, len: Option<usize>) -> Result<Self::MapState, Self::Error> {
        match len {
            Some(0) | None => {
                try!(self.writer.write_u8(M::TINY_MAP_NIBBLE | 0x00));
            }
            Some(len) => {
                if len <= M::USE_TINY_MAP as usize {
                    try!(self.writer.write_u8(M::TINY_MAP_NIBBLE | len as u8));
                } else if len <= M::USE_MAP_8 as usize {
                    try!(self.writer.write_u8(M::MAP_8));
                    try!(self.writer.write_u8(len as u8));
                } else if len <= M::USE_MAP_16 as usize {
                    try!(self.writer.write_u8(M::MAP_16));
                    try!(self.writer.write_u16::<BigEndian>(len as u16));
                } else if len <= M::USE_MAP_32 as usize {
                    try!(self.writer.write_u8(M::MAP_32));
                    try!(self.writer.write_u32::<BigEndian>(len as u32));
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_map_key<T: ser::Serialize>(
        &mut self,
        _state: &mut Self::MapState,
        key: T,
    ) -> Result<(), Self::Error> {
        key.serialize(self)
    }

    #[inline]
    fn serialize_map_value<T: ser::Serialize>(
        &mut self,
        _: &mut Self::MapState,
        value: T,
    ) -> Result<(), Self::Error> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_map_end(&mut self, _state: Self::MapState) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_struct(
        &mut self,
        name: &'static str,
        len: usize
    ) -> Result<Self::StructState, Self::Error> {
        if name == STRUCTURE_IDENTIFIER {
            if len <= M::USE_TINY_STRUCT {
                try!(self.writer.write_u8(M::TINY_STRUCT_NIBBLE | len as u8));
            } else if len <= M::USE_STRUCT_8 {
                try!(self.writer.write_u8(M::STRUCT_8));
                try!(self.writer.write_u8(len as u8));
            } else if len <= M::USE_STRUCT_16 {
                try!(self.writer.write_u8(M::STRUCT_16));
                try!(self.writer.write_u16::<BigEndian>(len as u16));
            } else {
                return Err(SerializerError::InvalidStructureLength)
            }
            Ok(StructState::Structure)
        } else {
            try!(self.serialize_map(Some(len)));
            Ok(StructState::Map)
        }
    }

    #[inline]
    fn serialize_struct_elt<V: ser::Serialize>(
        &mut self,
        state: &mut Self::StructState,
        key: &'static str,
        value: V
    ) -> Result<(), Self::Error> {
        if *state == StructState::Map {
            try!(key.serialize(self));
        }
        value.serialize(self)
    }

    #[inline]
    fn serialize_struct_end(&mut self, _state: Self::StructState) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_struct_variant(
        &mut self,
        name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        len: usize
    ) -> Result<Self::StructVariantState, Self::Error> {
        try!(self.serialize_map(Some(1)));
        try!(self.serialize_str(variant));
        self.serialize_struct(name, len)
    }

    #[inline]
    fn serialize_struct_variant_elt<V: ser::Serialize>(
        &mut self,
        state: &mut Self::StructVariantState,
        key: &'static str,
        value: V
    ) -> Result<(), Self::Error> {
        self.serialize_struct_elt(state, key, value)
    }

    #[inline]
    fn serialize_struct_variant_end(&mut self, state: Self::StructVariantState) -> Result<(), Self::Error> {
        self.serialize_struct_end(state)
    }
}

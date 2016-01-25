use std::error::Error;
use std::fmt;
use std::io;
use std::string;
use byteorder;
use serde::de::{self, Type};

#[derive(Debug)]
pub enum DecoderError {
    Io(io::Error),
    SyntaxError(String),
    UnexpectedType(&'static str),
    UnexpectedMarker(String, String),
    UnexpectedInput(String, String),
    UnknownVariant(String),
    UnknownField(String),
    MissingField(String),
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
            DecErr::UnexpectedMarker(ref exp, ref got) | DecErr::UnexpectedInput(ref exp, ref got) =>
                write!(f, "Expected '{}', Found '{}'", exp, got),
            DecErr::UnexpectedType(exp) =>
                write!(f, "Unexpected '{}'", exp),
            DecErr::WrongField(ref exp, ref got) =>
                write!(f, "Expected field '{}', Found '{}'", exp, got),
            _ => fmt::Debug::fmt(&self, f)
        }
    }
}

impl de::Error for DecoderError {
    fn syntax(msg: &str) -> Self {
        DecErr::SyntaxError(msg.to_owned())
    }

    fn type_mismatch(ty: Type) -> Self {
        match ty {
            Type::Bool => DecErr::UnexpectedType("bool"),
            Type::Usize => DecErr::UnexpectedType("usize"),
            Type::U8 => DecErr::UnexpectedType("u8"),
            Type::U16 => DecErr::UnexpectedType("u16"),
            Type::U32 => DecErr::UnexpectedType("u32"),
            Type::U64 => DecErr::UnexpectedType("u64"),
            Type::Isize => DecErr::UnexpectedType("isize"),
            Type::I8 => DecErr::UnexpectedType("i8"),
            Type::I16 => DecErr::UnexpectedType("i16"),
            Type::I32 => DecErr::UnexpectedType("i32"),
            Type::I64 => DecErr::UnexpectedType("i64"),
            Type::F32 => DecErr::UnexpectedType("f32"),
            Type::F64 => DecErr::UnexpectedType("f64"),
            Type::Char => DecErr::UnexpectedType("char"),
            Type::Str => DecErr::UnexpectedType("&str"),
            Type::String => DecErr::UnexpectedType("String"),
            Type::Unit => DecErr::UnexpectedType("()"),
            Type::Option => DecErr::UnexpectedType("Option<T>"),
            Type::Seq => DecErr::UnexpectedType("sequence type"),
            Type::Map => DecErr::UnexpectedType("map type"),
            Type::UnitStruct => DecErr::UnexpectedType("unit struct"),
            Type::NewtypeStruct => DecErr::UnexpectedType("newtype struct"),
            Type::TupleStruct => DecErr::UnexpectedType("tuple struct"),
            Type::Struct => DecErr::UnexpectedType("struct"),
            Type::Tuple => DecErr::UnexpectedType("tuple"),
            Type::Enum => DecErr::UnexpectedType("enum"),
            Type::StructVariant => DecErr::UnexpectedType("struct variant"),
            Type::TupleVariant => DecErr::UnexpectedType("tuple variant"),
            Type::UnitVariant => DecErr::UnexpectedType("unit variant"),
            Type::Bytes => DecErr::UnexpectedType("&[u8]"),
        }
    }

    fn end_of_stream() -> Self {
        DecoderError::UnexpectedType("EOF")
    }

    fn unknown_field(field: &str) -> Self {
        DecoderError::UnknownField(field.to_owned())
    }

    fn missing_field(field: &'static str) -> Self {
        DecoderError::MissingField(field.to_owned())
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

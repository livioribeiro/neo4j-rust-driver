use std::error::Error;
use std::fmt;
use std::io;
use std::string;
use serde::de;

#[derive(Debug)]
pub enum DeserializerError {
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

use self::DeserializerError as DesErr;

impl Error for DeserializerError {
    fn description(&self) -> &str { "deserializer error" }
}

impl fmt::Display for DeserializerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DesErr::UnexpectedMarker(ref exp, ref got) | DesErr::UnexpectedInput(ref exp, ref got) =>
                write!(f, "Expected '{}', Found '{}'", exp, got),
            DesErr::UnexpectedType(exp) =>
                write!(f, "Unexpected '{}'", exp),
            DesErr::WrongField(ref exp, ref got) =>
                write!(f, "Expected field '{}', Found '{}'", exp, got),
            _ => fmt::Debug::fmt(&self, f)
        }
    }
}

impl de::Error for DeserializerError {
    fn custom<T: Into<String>>(msg: T) -> Self {
        DeserializerError::SyntaxError(msg.into())
    }

    fn end_of_stream() -> Self {
        DeserializerError::UnexpectedType("EOF")
    }
}


impl From<io::Error> for DeserializerError {
    fn from(error: io::Error) -> Self {
        DesErr::Io(error)
    }
}

impl From<string::FromUtf8Error> for DeserializerError {
    fn from(_: string::FromUtf8Error) -> Self {
        DesErr::InvalidUTF8
    }
}

use std::error::Error;
use std::fmt;
use std::io;
use std::string;
use byteorder;
use serde::de;

#[derive(Debug)]
pub enum DecoderError {
    Io(io::Error),
    Syntax(String),
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

impl de::Error for DecoderError {
    fn syntax(msg: &str) -> Self {
        DecoderError::Syntax(msg.to_owned())
    }

    fn end_of_stream() -> Self {
        DecoderError::UnexpectedEOF
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

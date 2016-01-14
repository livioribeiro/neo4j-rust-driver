use std::collections::BTreeMap;
use std::io::prelude::*;
use std::io;
use byteorder::{self, ReadBytesExt, BigEndian};

use super::Value;
use super::super::marker as m;

enum ParserEvent {
    Null,
    True,
    False,
    Integer(i64),
    Float(f64),
    String(usize),
    List(usize),
    Map(usize),
    Struct(u8, usize),
}

use self::ParserEvent as ev;

type ParserEventResult = Result<ParserEvent, byteorder::Error>;

pub struct Parser<'a, R: Read + 'a> {
    reader: &'a mut R,
}

impl<'a, R: Read + 'a> Parser<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Parser {
            reader: reader,
        }
    }

    fn read_next(&mut self) -> ParserEventResult {
        match try!(self.reader.read_u8()) {
            m::NULL => Ok(ev::Null),
            m::TRUE => Ok(ev::True),
            m::FALSE => Ok(ev::False),
            v @ 0x00...0x7F => Ok(ev::Integer(v as i64)),
            v @ 0xF0...0xFF => Ok(ev::Integer((v | 0b1111_0000) as i64)),
            m::INT_8 => self.read_int(8),
            m::INT_16 => self.read_int(16),
            m::INT_32 => self.read_int(32),
            m::INT_64 => self.read_int(64),
            m::FLOAT => self.reader.read_f64::<BigEndian>().map(|v| ev::Float(v)),
            v @ 0x80...0x8F => Ok(ev::String((v & 0b0000_1111) as usize)),
            m::STRING_8 => self.read_len(8).map(|v| ev::String(v)),
            m::STRING_16 => self.read_len(16).map(|v| ev::String(v)),
            m::STRING_32 => self.read_len(32).map(|v| ev::String(v)),
            v @ 0x90...0x9F => Ok(ev::List((v & 0b0000_1111) as usize)),
            m::LIST_8 => self.read_len(8).map(|v| ev::List(v)),
            m::LIST_16 => self.read_len(16).map(|v| ev::List(v)),
            m::LIST_32 => self.read_len(32).map(|v| ev::List(v)),
            v @ 0xA0...0xAF => Ok(ev::Map((v & 0b0000_1111) as usize)),
            m::MAP_8 => self.read_len(8).map(|v| ev::Map(v)),
            m::MAP_16 => self.read_len(16).map(|v| ev::Map(v)),
            m::MAP_32 => self.read_len(32).map(|v| ev::Map(v)),
            v @ 0xB0...0xBF => self.reader.read_u8().map(|s| ev::Struct(s, (v & 0b0000_1111) as usize)),
            m::STRUCT_8 => self.reader.read_u8().and_then(|s| self.read_len(8).map(|v| ev::Struct(s, v))),
            m::STRUCT_16 => self.reader.read_u8().and_then(|s| self.read_len(16).map(|v| ev::Struct(s, v))),
            _ => unreachable!()
        }
    }

    fn read_int(&mut self, size: u8) -> ParserEventResult {
        match size {
            8 => self.reader.read_i8().map(|v| ev::Integer(v as i64)),
            16 => self.reader.read_i16::<BigEndian>().map(|v| ev::Integer(v as i64)),
            32 => self.reader.read_i32::<BigEndian>().map(|v| ev::Integer(v as i64)),
            64 => self.reader.read_i64::<BigEndian>().map(|v| ev::Integer(v as i64)),
            _ => unreachable!(),
        }
    }

    fn read_len(&mut self, size: usize) -> Result<usize, byteorder::Error> {
        match size {
            8 => self.reader.read_i8().map(|v| v as usize),
            16 => self.reader.read_i16::<BigEndian>().map(|v| v as usize),
            32 => self.reader.read_i32::<BigEndian>().map(|v| v as usize),
            _ => unreachable!(),
        }
    }

    fn read_string(&mut self, size: usize) -> Result<String, byteorder::Error> {
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

        String::from_utf8(store).map_err(|e| byteorder::Error::UnexpectedEOF)
    }
}

impl<'a, R: Read + 'a> Iterator for Parser<'a, R> {
    type Item = Result<Value, byteorder::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.read_next() {
            Ok(e) => match e {
                ev::Null => Value::Null,
                ev::True => Value::Boolean(true),
                ev::False => Value::Boolean(false),
                ev::Integer(v) => Value::Integer(v),
                ev::Float(v) => Value::Float(v),
                ev::String(size) => {
                    let res = self.read_string(size).map(|s| Value::String(s));
                    if res.is_err() { return Some(res) }
                    res.unwrap()
                },
                ev::List(size) => Value::List(Vec::with_capacity(size)),
                ev::Map(size) => Value::Map(BTreeMap::new()),
                ev::Struct(s, size) => Value::Structure(s, Vec::with_capacity(size)),
            }
        };
    }
}

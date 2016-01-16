use std::collections::BTreeMap;
use std::io::prelude::*;
use byteorder::{ReadBytesExt, BigEndian};

use super::Value;
use super::super::deserialize::{DecoderError, DecodeResult};
use super::super::marker as m;

pub fn from_reader<'a, R: Read + 'a>(reader: &mut R) -> DecodeResult<Value> {
    let mut builder = Builder::new(reader);
    builder.build()
}

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

type ParserEventResult = DecodeResult<ParserEvent>;

pub struct Builder<'a, R: Read + 'a> {
    reader: &'a mut R,
    stack: Vec<Value>,
}

impl<'a, R: Read + 'a> Builder<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Builder {
            reader: reader,
            stack: Vec::new(),
        }
    }

    pub fn build(&mut self) -> DecodeResult<Value> {
        try!(self.parse());
        Ok(self.stack.pop().unwrap_or(Value::Null))
    }

    pub fn parse(&mut self) -> DecodeResult<()> {
        let mut buf = [0u8; 1];
        let bytes_read = try!(self.reader.read(&mut buf));

        if bytes_read == 0 {
            return Ok(())
        }

        match self.read_next(buf[0]) {
            Ok(e) => match e {
                ev::Null => self.stack.push(Value::Null),
                ev::True => self.stack.push(Value::Boolean(true)),
                ev::False => self.stack.push(Value::Boolean(false)),
                ev::Integer(v) => self.stack.push(Value::Integer(v)),
                ev::Float(v) => self.stack.push(Value::Float(v)),
                ev::String(size) => {
                    let value = try!(self.read_string(size));
                    self.stack.push(Value::String(value));
                },
                ev::List(size) => {
                    let values = {
                        let mut values = vec![];
                        for _ in 0..size {
                            // println!("{}", i);
                            try!(self.parse());
                            match self.stack.pop() {
                                Some(v) => values.push(v),
                                _ => return Err(DecoderError::UnexpectedEOF)
                            }
                        }
                        values

                    };
                    self.stack.push(Value::List(values));
                },
                ev::Map(size) => {
                    let size = size * 2;
                    let values = {
                        let mut cur_key: String = String::new();
                        let mut values: BTreeMap<String, Value> = BTreeMap::new();

                        for i in 1..(size + 1) {
                            try!(self.parse());
                            match self.stack.pop() {
                                Some(Value::String(ref k)) if i % 2 != 0 => cur_key = k.to_owned(),
                                Some(ref v) if i % 2 != 0 => return Err(DecoderError::UnexpectedInput(
                                    "Map key".to_owned(), format!("{:?}", v)
                                )),
                                Some(v) => {
                                    if cur_key.is_empty() {
                                        return Err(DecoderError::UnexpectedInput(
                                            "Map key".to_owned(), "None".to_owned()
                                        ))
                                    }
                                    values.insert(cur_key.clone(), v);
                                },
                                _ => return Err(DecoderError::UnexpectedEOF),
                            }
                        }

                        values
                    };
                    self.stack.push(Value::Map(values));
                },
                ev::Struct(s, size) => {
                    let values = {
                        let mut values = vec![];
                        for _ in 0..size {
                            try!(self.parse());
                            match self.stack.pop() {
                                Some(v) => values.push(v),
                                _ => return Err(DecoderError::UnexpectedEOF)
                            }
                        }
                        values

                    };
                    self.stack.push(Value::Structure(s, values));
                },
            },
            Err(e) => return Err(From::from(e))
        };

        Ok(())
    }

    fn read_next(&mut self, marker: u8) -> ParserEventResult {
        match marker {
            m::NULL => Ok(ev::Null),
            m::TRUE => Ok(ev::True),
            m::FALSE => Ok(ev::False),
            v @ 0x00...0x7F => Ok(ev::Integer(v as i64)),
            v @ 0xF0...0xFF => Ok(ev::Integer(((v | 0b1111_0000) as i8) as i64)),
            m::INT_8 => self.read_int(8),
            m::INT_16 => self.read_int(16),
            m::INT_32 => self.read_int(32),
            m::INT_64 => self.read_int(64),
            m::FLOAT => self.reader.read_f64::<BigEndian>().map(
                |v| ev::Float(v)).map_err(From::from),
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
            v @ 0xB0...0xBF => self.reader.read_u8().map(
                |s| ev::Struct(s, (v & 0b0000_1111) as usize)).map_err(From::from),
            m::STRUCT_8 => self.read_len(8)
                .map_err(From::from)
                .and_then(|size| self.reader.read_u8()
                    .map(|sig| ev::Struct(sig, size))
                    .map_err(From::from)),
            m::STRUCT_16 => self.read_len(16)
                .map_err(From::from)
                .and_then(|size| self.reader.read_u8()
                    .map(|sig| ev::Struct(sig, size))
                    .map_err(From::from)),
            _ => unreachable!()
        }
    }

    fn read_int(&mut self, size: u8) -> ParserEventResult {
        match size {
            8 => self.reader.read_i8().map(|v| ev::Integer(v as i64)).map_err(From::from),
            16 => self.reader.read_i16::<BigEndian>().map(|v| ev::Integer(v as i64)).map_err(From::from),
            32 => self.reader.read_i32::<BigEndian>().map(|v| ev::Integer(v as i64)).map_err(From::from),
            64 => self.reader.read_i64::<BigEndian>().map(|v| ev::Integer(v as i64)).map_err(From::from),
            _ => unreachable!(),
        }
    }

    fn read_len(&mut self, size: usize) -> DecodeResult<usize> {
        match size {
            8 => self.reader.read_u8().map(|v| v as usize).map_err(From::from),
            16 => self.reader.read_u16::<BigEndian>().map(|v| v as usize).map_err(From::from),
            32 => self.reader.read_u32::<BigEndian>().map(|v| v as usize).map_err(From::from),
            _ => unreachable!(),
        }
    }

    fn read_string(&mut self, size: usize) -> DecodeResult<String> {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::string::String;
    use std::io::Cursor;
    use super::from_reader;
    use super::super::Value;
    use ::v1::packstream::marker as m;

    #[test]
    fn decode_nil() {
        let mut input = Cursor::new(vec![0xC0]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Null, result);
    }

    #[test]
    fn decode_bool() {
        let mut input = Cursor::new(vec![0xC3]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Boolean(true), result);

        let mut input = Cursor::new(vec![0xC2]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Boolean(false), result);
    }

    // Integer 64
    #[test]
    fn decode_int64_positive() {
        let mut input = Cursor::new(vec![m::INT_64, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_POS_INT_64.1), result);
    }

    #[test]
    fn decode_int64_negative() {
        let mut input = Cursor::new(vec![m::INT_64, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_NEG_INT_64.0), result);
    }

    // Integer 32
    #[test]
    fn decode_int32_positive() {
        let mut input = Cursor::new(vec![m::INT_32, 0x7F, 0xFF, 0xFF, 0xFF]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_POS_INT_32.1), result);
    }

    #[test]
    fn decode_int32_negative() {
        let mut input = Cursor::new(vec![m::INT_32, 0x80, 0x00, 0x00, 0x00]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_NEG_INT_32.0), result);
    }

    // Integer 16
    #[test]
    fn decode_int16_positive() {
        let mut input = Cursor::new(vec![m::INT_16, 0x7F, 0xFF]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_POS_INT_16.1), result);
    }

    #[test]
    fn decode_int16_negative() {
        let mut input = Cursor::new(vec![m::INT_16, 0x80, 0x00]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_NEG_INT_16.0), result);
    }

    // Integer 8
    #[test]
    fn decode_int8_positive() {
        let mut input = Cursor::new(vec![0x7F]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_TINY_INT.1), result);
    }

    #[test]
    fn decode_int8_negative() {
        let mut input = Cursor::new(vec![m::INT_8, 0x80]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_NEG_INT_8.0), result);

        let mut input = Cursor::new(vec![0xF0]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Integer(m::RANGE_TINY_INT.0), result);
    }

    #[test]
    fn decode_float_positive() {
        let mut input = Cursor::new(vec![m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Float(1.1), result);
    }

    #[test]
    fn decode_float_negative() {
        let mut input = Cursor::new(vec![m::FLOAT, 0xBF, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]);
        let result = from_reader(&mut input).unwrap();
        assert_eq!(Value::Float(-1.1), result);
    }

    #[test]
    fn decode_string32() {
        let size = 70_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRING_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, _| { acc.push(b'A'); acc }
        ));

        let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
        let result = from_reader(&mut input).unwrap();

        assert_eq!(Value::String(expected), result);
    }

    #[test]
    fn decode_string16() {
        let size = 5_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRING_16, 0x13, 0x88],
            |mut acc, _| { acc.push(b'A'); acc }
        ));

        let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
        let result = from_reader(&mut input).unwrap();

        assert_eq!(Value::String(expected), result);
    }

    #[test]
    fn decode_string8() {
        let size = 200;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRING_8, 0xC8],
            |mut acc, _| { acc.push(b'A'); acc }
        ));

        let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
        let result = from_reader(&mut input).unwrap();

        assert_eq!(Value::String(expected), result);
    }

    #[test]
    fn decode_tiny_string() {
        for marker in 0x80..0x8F {
            let size = marker - m::TINY_STRING_NIBBLE;
            let mut input = Cursor::new((0..size).fold(
                vec![marker],
                |mut acc, _| { acc.push(b'A'); acc }
            ));

            let expected = (0..size).fold(String::new(), |mut acc, _| { acc.push('A'); acc });
            let result = from_reader(&mut input).unwrap();

            assert_eq!(Value::String(expected), result);
        }
    }

    #[test]
    fn decode_char() {
        for c in b'A'..b'Z' {
            let mut input = Cursor::new(vec![0x81, c]);
            let result = from_reader(&mut input).unwrap();

            assert_eq!(Value::String(format!("{}", c as char)), result);
        }
    }

    #[test]
    fn decode_list32() {
        let size = 70_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::LIST_32, 0x00, 0x01, 0x11, 0x70],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = Value::List(vec![Value::Integer(1); size]);
        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_list16() {
        let size = 5_000;
        let mut input = Cursor::new((0..size).fold(
            vec![m::LIST_16, 0x13, 0x88],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = Value::List(vec![Value::Integer(1); size]);
        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_list8() {
        let size = 200;
        let mut input = Cursor::new((0..size).fold(
            vec![m::LIST_8, 0xC8],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = Value::List(vec![Value::Integer(1); size]);
        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_tiny_list() {
        for marker in 0x90..0x9F {
            let size = (marker - m::TINY_LIST_NIBBLE) as usize;
            let mut input = Cursor::new((0..size).fold(
                vec![marker],
                |mut acc, _| { acc.push(0x01); acc }
            ));

            let expected = Value::List(vec![Value::Integer(1); size]);
            let result = from_reader(&mut input).unwrap();

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn decode_list_of_string() {
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

        let result = from_reader(&mut input).unwrap();
        let expected = Value::List(vec![Value::String("abcdefghijklmnopqrstuvwxyz".to_owned()); size]);

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_list_of_int() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 m::INT_16, 0x7D, 0x00,
                 m::INT_16, 0x7D, 0x00,
                 m::INT_16, 0x7D, 0x00]
             );

        let result = from_reader(&mut input).unwrap();
        let expected = Value::List(vec![Value::Integer(32_000); size]);

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_list_of_float() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]
             );

        let result = from_reader(&mut input).unwrap();
        let expected = Value::List(vec![Value::Float(1.1); size]);

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_list_of_bool() {
        let size = 4;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 m::TRUE, m::FALSE, m::TRUE, m::FALSE]
             );

        let result = from_reader(&mut input).unwrap();
        let expected = Value::List(vec![Value::Boolean(true),
                                        Value::Boolean(false),
                                        Value::Boolean(true),
                                        Value::Boolean(false)]);

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_mixed_list() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_LIST_NIBBLE + size as u8,
                 0x01,
                 m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 m::TINY_STRING_NIBBLE + 1, 0x41]
             );

        let result = from_reader(&mut input).unwrap();
        let expected = Value::List(vec![Value::Integer(1),
                                        Value::Float(1.1),
                                        Value::String("A".to_owned())]);

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map32() {
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

        let expected = Value::Map((0..size).fold(
            BTreeMap::<String, Value>::new(),
            |mut acc, i| { acc.insert(format!("{:05}", i), Value::Integer(1)); acc }
        ));

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map16() {
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

        let expected = Value::Map((0..size).fold(
            BTreeMap::<String, Value>::new(),
            |mut acc, i| { acc.insert(format!("{:04}", i), Value::Integer(1)); acc }
        ));

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map8() {
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

        let expected = Value::Map((0..size).fold(
            BTreeMap::<String, Value>::new(),
            |mut acc, i| { acc.insert(format!("{:03}", i), Value::Integer(1)); acc }
        ));

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_tiny_map() {
        let size = 3;

        let mut input = Cursor::new((0..size).fold(
            vec![m::TINY_MAP_NIBBLE + size],
            |mut acc, i| {
                acc.extend([0x81, 0x30 + i].iter());
                acc.push(0x01);
                acc
            }
        ));

        let expected = Value::Map((0..size).fold(
            BTreeMap::<String, Value>::new(),
            |mut acc, i| { acc.insert(format!("{}", i), Value::Integer(1)); acc }
        ));

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map_of_string() {
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
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();
            expected.insert("A".to_owned(), Value::String("abcdefghijklmnopqrstuvwxyz".to_owned()));
            expected.insert("B".to_owned(), Value::String("abcdefghijklmnopqrstuvwxyz".to_owned()));
            expected.insert("C".to_owned(), Value::String("abcdefghijklmnopqrstuvwxyz".to_owned()));
            Value::Map(expected)
        };

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map_of_int() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
                 0x81, 0x41, m::INT_16, 0x7D, 0x00,
                 0x81, 0x42, m::INT_16, 0x7D, 0x00,
                 0x81, 0x43, m::INT_16, 0x7D, 0x00]
        );

        let expected = {
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();
            expected.insert("A".to_owned(), Value::Integer(32_000));
            expected.insert("B".to_owned(), Value::Integer(32_000));
            expected.insert("C".to_owned(), Value::Integer(32_000));
            Value::Map(expected)
        };

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map_of_float() {
        let size = 3;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
                 0x81, 0x41, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 0x81, 0x42, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
                 0x81, 0x43, m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A]
        );

        let expected = {
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();
            expected.insert("A".to_owned(), Value::Float(1.1));
            expected.insert("B".to_owned(), Value::Float(1.1));
            expected.insert("C".to_owned(), Value::Float(1.1));
            Value::Map(expected)
        };

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_map_of_bool() {
        let size = 4;

        let mut input = Cursor::new(
            vec![m::TINY_MAP_NIBBLE + size,
                 0x81, 0x41, m::TRUE,
                 0x81, 0x42, m::FALSE,
                 0x81, 0x43, m::TRUE,
                 0x81, 0x44, m::FALSE]
        );

        let expected = {
            let mut expected: BTreeMap<String, Value> = BTreeMap::new();
            expected.insert("A".to_owned(), Value::Boolean(true));
            expected.insert("B".to_owned(), Value::Boolean(false));
            expected.insert("C".to_owned(), Value::Boolean(true));
            expected.insert("D".to_owned(), Value::Boolean(false));
            Value::Map(expected)
        };

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_tiny_structure() {
        let mut input = Cursor::new(vec![m::TINY_STRUCT_NIBBLE + 0x03, 0x22,
            0x01,
            m::FLOAT, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A,
            0x81, 0x41
        ]);

        let expected = Value::Structure(0x22, vec![Value::Integer(1),
                                                   Value::Float(1.1),
                                                   Value::String("A".to_owned())]);

        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_structure8() {
        let size = 16;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRUCT_8, 0x10, 0x22],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = Value::Structure(0x22, vec![Value::Integer(1); size]);
        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn decode_structure16() {
        let size = 256;
        let mut input = Cursor::new((0..size).fold(
            vec![m::STRUCT_16, 0x01, 0x00, 0x22],
            |mut acc, _| { acc.push(0x01); acc }
        ));

        let expected = Value::Structure(0x22, vec![Value::Integer(1); size]);
        let result = from_reader(&mut input).unwrap();

        assert_eq!(expected, result);
    }
}

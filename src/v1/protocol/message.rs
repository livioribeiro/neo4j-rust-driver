use std::convert::Into;
use rustc_serialize::{Encodable, Encoder};

use ::v1::packstream::value::{Value, Map};

const INIT_SIZE: usize = 1;
const RUN_SIZE: usize = 2;

pub struct Init {
    client_name: String,
}

impl Init {
    pub fn new(client_name: &str) -> Self {
        Init {
            client_name: client_name.into(),
        }
    }
}

impl Encodable for Init {
    fn encode<S: Encoder>(&self, e: &mut S) -> Result<(), S::Error> {
        try!(e.emit_struct("__STRUCTURE__INIT", INIT_SIZE as usize, |_| Ok(())));
        try!(self.client_name.encode(e));

        Ok(())
    }
}

pub struct Run {
    statement: String,
    parameters: Map,
}

impl Run {
    pub fn new(statement: &str) -> Self {
        Run {
            statement: statement.to_owned(),
            parameters: Map::new(),
        }
    }

    pub fn add_param<T: Into<Value>>(&mut self, name: &str, value: T) {
        self.parameters.insert(name.to_owned(), value.into());
    }

    pub fn with_param<T: Into<Value>>(mut self, name: &str, value: T) -> Self {
        self.add_param(name, value);
        self
    }
}

impl Encodable for Run {
    fn encode<S: Encoder>(&self, e: &mut S) -> Result<(), S::Error> {
        try!(e.emit_struct("__STRUCTURE__RUN", RUN_SIZE, |_| Ok(())));
        try!(self.statement.encode(e));
        try!(self.parameters.encode(e));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Init, Run};
    use ::v1::packstream::serialize::encode;

    #[test]
    fn serialize_init() {
        let input = Init::new("MyClient/1.0".into());

        let result = encode(&input).unwrap();
        let expected = vec![0xB1, 0x01, 0x8C, 0x4D,
                            0x79, 0x43, 0x6C, 0x69,
                            0x65, 0x6E, 0x74, 0x2F,
                            0x31, 0x2E, 0x30];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_run() {
        let input = Run::new("RETURN 1 AS num");

        let result = encode(&input).unwrap();
        let expected = vec![0xB2, 0x10, 0x8F, 0x52,
                            0x45, 0x54, 0x55, 0x52,
                            0x4E, 0x20, 0x31, 0x20,
                            0x41, 0x53, 0x20, 0x6E,
                            0x75, 0x6D, 0xA0];

        assert_eq!(expected, result);
    }

    #[test]
    fn serialize_run_with_parameters() {
        let input = Run::new("CREATE (n {p: {v}})").with_param("v", 1);

        let result = encode(&input).unwrap();
        let expected = vec![0xB2, 0x10, 0xD0, 0x13,
                            0x43, 0x52, 0x45, 0x41,
                            0x54, 0x45, 0x20, 0x28,
                            0x6E, 0x20, 0x7B, 0x70,
                            0x3A, 0x20, 0x7B, 0x76,
                            0x7D, 0x7D, 0x29,
                            0xA1, 0x81, 0x76, 0x01];

        assert_eq!(expected, result);
    }
}

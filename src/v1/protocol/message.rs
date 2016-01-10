use rustc_serialize::{Encodable, Encoder};

const INIT_SIZE: usize = 1;

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
        self.client_name.encode(e)
    }
}

#[cfg(test)]
mod tests {
    use super::Init;
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
}

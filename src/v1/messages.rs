use byteorder;

use super::packstream::marker;

const INIT_SIG: u8 = 0x01;

pub struct Init {
    client_name: String,
}

impl Init {
    pub fn new(client_name: &str) -> Self {
        Init {
            client_name: client_name.into(),
        }
    }

    pub fn encode(self) -> Result<Vec<u8>, byteorder::Error> {
        let struct_marker = marker::TINY_STRUCT_NIBBLE + 1;
        let mut data = try!(super::packstream::encode(&self.client_name));
        let mut message = vec![struct_marker, INIT_SIG];
        message.append(&mut data);
        Ok(message)
    }
}

use super::packstream::markers;

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

    pub fn encode(self) -> Vec<u8> {
        let struct_marker = markers::TINY_STRUCT_NIBBLE + 1;
        let mut message = vec![struct_marker, INIT_SIG];
        message.append(&mut Vec::from(self.client_name.as_bytes()));
        message
    }
}

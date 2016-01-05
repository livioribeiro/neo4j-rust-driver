use std::io::prelude::*;
use std::net::TcpStream;
// use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

pub struct Connection {
    socket: TcpStream,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Connection {
            socket: socket,
        }
    }

    pub fn init(&mut self, user_agent: &str) -> Vec<u8> {
        let mut data: Vec<u8> = vec![0xb1, 0x01];
        data.push(0b1000_0000 + user_agent.len() as u8);
        data.append(&mut Vec::from(user_agent.as_bytes()));

        let mut info = String::new();
        for b in data.iter() {
            info.push_str(format!("{:02.X} ", b).as_ref());
        }

        info!("Sending init message: {}", &info);

        self.socket.write(&data).unwrap();
        self.socket.flush().unwrap();

        let mut buf = Vec::new();
        self.socket.read_to_end(&mut buf).unwrap();

        buf
    }
}

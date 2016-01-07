use std::net::TcpStream;

use super::transport::ChunkedStream;
use super::messages::Init;

pub struct Connection {
    transport: ChunkedStream,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Connection {
            transport: ChunkedStream::new(socket),
        }
    }

    pub fn init(&mut self, user_agent: &str) -> Vec<u8> {
        let message = Init::new(user_agent);
        let data = message.encode();

        self.transport.write(&data).unwrap();

        let mut info = String::new();
        for b in self.transport.raw().iter() {
            info.push_str(format!("{:02X} ", b).as_ref());
        }

        info!("Sending init message: {}", &info);

        self.transport.flush(true).unwrap();
        self.transport.send().unwrap();

        vec![]
    }
}

use std::net::TcpStream;
use byteorder;

use super::transport::ChunkedStream;
use super::protocol::message::Init;
use super::packstream::serialize::encode;

pub struct Connection {
    transport: ChunkedStream,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Connection {
            transport: ChunkedStream::new(socket),
        }
    }

    pub fn init(&mut self, user_agent: &str) -> Result<Vec<u8>, byteorder::Error> {
        let message = Init::new(user_agent);
        let data = try!(encode(&message));

        try!(self.transport.write(&data));

        try!(self.transport.flush(true));
        try!(self.transport.send());

        Ok(vec![])
    }
}

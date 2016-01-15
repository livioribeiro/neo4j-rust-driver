use std::collections::BTreeMap;
use std::io::Cursor;
use std::net::TcpStream;

use super::transport::ChunkedStream;
use super::protocol::client::{Init, Run, PullAll};
use super::protocol::server::Message;
use super::packstream::{encode, decode};

pub struct Connection {
    transport: ChunkedStream,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Connection {
            transport: ChunkedStream::new(socket),
        }
    }

    pub fn init(&mut self, user_agent: &str) -> Message<BTreeMap<String, ()>> {
        let message = Init::new(user_agent);
        let data = encode(&message).unwrap();

        self.transport.write(&data).unwrap();

        self.transport.flush(true).unwrap();
        self.transport.send().unwrap();

        let data = self.transport.receive().unwrap();

        println!("[{}]", data.iter().fold(String::new(), |mut acc, i| { acc.push_str(&format!("{:02X} ", i)); acc }));

        let mut cur = Cursor::new(data);
        let msg: Message<BTreeMap<String, ()>> = decode(&mut cur).unwrap();
        msg
    }

    pub fn run(&mut self, query: &str) -> Message<BTreeMap<String, Vec<String>>> {
        let message = Run::new(query);
        let data = encode(&message).unwrap();

        self.transport.write(&data).unwrap();

        self.transport.flush(true).unwrap();
        self.transport.send().unwrap();

        let data = self.transport.receive().unwrap();

        println!("[ {}]", data.iter().fold(String::new(), |mut acc, i| { acc.push_str(&format!("{:02X} ", i)); acc }));

        let mut cur = Cursor::new(data);
        let msg: Message<BTreeMap<String, Vec<String>>> = decode(&mut cur).unwrap();
        msg
    }

    pub fn pull_all(&mut self) -> Message<Vec<u32>> {
        let message = PullAll;
        let data = encode(&message).unwrap();

        self.transport.write(&data).unwrap();

        self.transport.flush(true).unwrap();
        self.transport.send().unwrap();

        let data = self.transport.receive().unwrap();

        println!("[ {}]", data.iter().fold(String::new(), |mut acc, i| { acc.push_str(&format!("{:02X} ", i)); acc }));

        let mut cur = Cursor::new(data);
        let msg: Message<Vec<u32>> = decode(&mut cur).unwrap();
        msg
    }
}

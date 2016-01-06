use std::io::prelude::*;
use std::io::{self, Cursor};
use std::net::TcpStream;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

const MAX_CHUNK_SIZE: usize = 65535;

pub struct ChunkedStream {
    socket: TcpStream,
    raw: Cursor<Vec<u8>>,
    output_buffer: Vec<u8>,
    output_size: usize,
}

impl ChunkedStream {
    pub fn new(socket: TcpStream) -> Self {
        ChunkedStream {
            socket: socket,
            raw: Cursor::new(Vec::new()),
            output_buffer: Vec::new(),
            output_size: 0,
        }
    }
}

// based on https://github.com/neo4j/neo4j-python-driver/blob/1.0/neo4j/v1/connection.py
impl Write for ChunkedStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut b = buf;

        loop {
            let size = b.len();
            let future_size = self.output_size + size;

            if future_size >= MAX_CHUNK_SIZE {
                let end = MAX_CHUNK_SIZE - self.output_size;
                for i in b[0..end].iter() {
                    self.output_buffer.push(*i);
                }
                self.output_size = MAX_CHUNK_SIZE;
                b = &b[end..size];
                try!(self.flush());
            } else {
                for i in b.iter() {
                    self.output_buffer.push(*i);
                    self.output_size = future_size;
                }
                break
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.output_buffer.len() > 0 {
            let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
            try!(buf.write_u16::<BigEndian>(self.output_size as u16));
            try!(buf.write_all(self.output_buffer.as_ref()));

            try!(self.raw.write_all(buf.into_inner().as_ref()));
            try!(self.raw.flush());

            self.output_buffer.clear();
            self.output_size = 0;
        }

        Ok(())
    }
}

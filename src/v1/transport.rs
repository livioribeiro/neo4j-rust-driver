use std::io::prelude::*;
use std::io::{self, Cursor};
use std::net::TcpStream;
use byteorder::{self, ReadBytesExt, WriteBytesExt, BigEndian};

const MAX_CHUNK_SIZE: usize = 65535;

pub struct ChunkedStream {
    socket: TcpStream,
    raw: Cursor<Vec<u8>>,
    output_buffer: Vec<u8>,
    output_size: usize,
}

// based on https://github.com/neo4j/neo4j-python-driver/blob/1.0/neo4j/v1/connection.py
impl ChunkedStream {
    pub fn new(socket: TcpStream) -> Self {
        ChunkedStream {
            socket: socket,
            raw: Cursor::new(Vec::new()),
            output_buffer: Vec::new(),
            output_size: 0,
        }
    }

    pub fn raw(&self) -> &[u8] {
        self.raw.get_ref()
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<()> {
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
                try!(self.flush(false));
            } else {
                for i in b.iter() {
                    self.output_buffer.push(*i);
                    self.output_size = future_size;
                }
                break
            }
        }

        Ok(())
    }

    pub fn flush(&mut self, end_of_message: bool) -> io::Result<()> {
        let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());

        if self.output_buffer.len() > 0 {
            try!(buf.write_u16::<BigEndian>(self.output_size as u16));
            try!(buf.write_all(self.output_buffer.as_ref()));
        }

        if end_of_message {
            try!(buf.write(&[0x00, 0x00]));
        }

        if buf.get_ref().len() > 0 {
            try!(self.raw.write_all(buf.into_inner().as_ref()));
            try!(self.raw.flush());

            self.output_buffer.clear();
            self.output_size = 0;
        }

        Ok(())
    }

    pub fn send(&mut self) -> io::Result<()> {
        try!(self.socket.write_all(self.raw.get_ref()));

        debug!("C:{}", self.raw.get_ref().iter().fold(
            String::new(), |acc, i| format!("{} {:02X}", acc, i)
        ));

        self.raw.get_mut().clear();
        self.raw.set_position(0);
        Ok(())
    }

    pub fn receive(&mut self) -> io::Result<Vec<u8>> {
        let mut result: Vec<u8> = Vec::new();

        loop {
            let chunk_size = match self.socket.read_u16::<BigEndian>() {
                Ok(value) => value,
                Err(byteorder::Error::UnexpectedEOF) => break,
                Err(byteorder::Error::Io(e)) => return Err(e),
            };

            if chunk_size == 0 { break }

            let mut buf = vec![0u8; chunk_size as usize];
            try!(self.socket.read(&mut buf));
            result.append(&mut buf);
        }

        Ok(result)
    }
}

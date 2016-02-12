extern crate byteorder;
extern crate rustc_serialize;
extern crate url;

#[macro_use]
extern crate log;

pub mod v1;

use std::io::prelude::*;
use std::io::Cursor;
use std::net::{TcpStream, Shutdown};
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

use v1::Connection;

const PREAMBLE: [u8; 4] = [0x60, 0x60, 0xB0, 0x17];
const SUPPORTED_VERSIONS: [u32; 4] = [1, 0, 0, 0];

/// Connect and perform a handshake in order to return a valid
/// Connection object if a protocol version can be agreed.
pub fn connect(host: &str, port: u16) -> Result<Connection, ()> {
    info!("Creating connection to {} on port {}", host, port);

    let mut stream = TcpStream::connect((host, port)).unwrap();
    info!("Supported protocols are: {:?}", &SUPPORTED_VERSIONS);

    let data = {
        let mut data = Cursor::new(vec![0u8; (4 * 4)]);

        for i in PREAMBLE.iter() {
            data.write_u8(*i).unwrap();
        }

        for v in SUPPORTED_VERSIONS.iter() {
            data.write_u32::<BigEndian>(*v).unwrap();
        }

        data.into_inner()
    };

    debug!("Sending handshake data: {:?}", &data);
    stream.write(&data).unwrap();

    let mut buf = [0u8; 4];
    stream.read(&mut buf).unwrap();
    debug!("Received handshake data: {:?}", &buf);

    let agreed_version = {
        let mut data = Cursor::new(&buf);
        data.read_u32::<BigEndian>().unwrap()
    };

    if agreed_version == 0 {
        warn!("Closing connection as no protocol version could be agreed");
        stream.shutdown(Shutdown::Both).unwrap();

        return Err(())
    }

    info!("Protocol version {} agreed", agreed_version);
    Ok(Connection::new(stream))
}

extern crate byteorder;

#[macro_use]
extern crate log;

use std::io::prelude::*;
use std::io::Cursor;
use std::net::{TcpStream, Shutdown};
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

/// Connect and perform a handshake in order to return a valid
/// Connection object if a protocol version can be agreed.
pub fn connect(host: &str, port: u16) -> Result<u32, ()> {
    info!("Creating connection to {} on port {}", host, port);

    let mut stream = TcpStream::connect((host, port)).unwrap();
    let supported_version: [u32; 4] = [1, 0, 0, 0];
    info!("Supported protocols are: {:?}", &supported_version);

    let data = {
        let mut data = Cursor::new(vec![0u8; (4 * 4)]);
        for v in supported_version.iter() {
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
    Ok(agreed_version)
}

#[test]
fn connection() {
    let protocol_version = connect("localhost", 7687).unwrap();
    assert_eq!(1, protocol_version);
}

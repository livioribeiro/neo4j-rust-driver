use rustc_serialize::{Decodable, Decoder};

const RECORD_SIG: u8 = 0x71;
const SUCCESS_SIG: u8 = 0x70;
const FAILURE_SIG: u8 = 0x7F;
const IGNORED_SIG: u8 = 0x7E;

#[derive(Debug)]
pub enum ServerMessage {
    Record,
    Success,
    Failure,
    Ignored,
    Unknown(u8),
}

#[derive(Debug)]
pub struct Message<T: Decodable> {
    kind: ServerMessage,
    data: T,
}

impl<T: Decodable> Decodable for Message<T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let mut msg_type = ServerMessage::Unknown(0);
        let mut data: Option<T> = None;

        try!(d.read_struct("Message", 2, |d| {
            match try!(d.read_u8()) { // reading signature
                RECORD_SIG => msg_type = ServerMessage::Record,
                SUCCESS_SIG => msg_type = ServerMessage::Success,
                FAILURE_SIG => msg_type = ServerMessage::Failure,
                IGNORED_SIG => msg_type = ServerMessage::Ignored,
                v @ _ => msg_type = ServerMessage::Unknown(v),
            }
            let result = try!(T::decode(d));
            data = Some(result);
            Ok(())
        }));

        Ok(Message { kind: msg_type, data: data.unwrap() })
    }
}

// #[derive(Debug, RustcDecodable)]
// pub struct Record<T: Decodable> {
//     pub signature: u8,
//     pub fields: T,
// }
//
// #[derive(Debug, RustcDecodable)]
// pub struct Success<T: Decodable> {
//     pub signature: u8,
//     pub metadata: T,
// }
//
// #[derive(Debug, RustcDecodable)]
// pub struct Failure<T: Decodable> {
//     pub signature: u8,
//     pub metadata: T,
// }
//
// #[derive(Debug, RustcDecodable)]
// pub struct Ignored<T: Decodable> {
//     pub signature: u8,
//     pub metadata: T,
// }

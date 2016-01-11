pub mod message;

pub const INIT: u8 = 0x01;
pub const RUN: u8 = 0x10;

pub fn signature(name: &str) -> Option<u8> {
    let sig = match name {
        "INIT" => INIT,
        "RUN" => RUN,
        _ => return None,
    };

    Some(sig)
}

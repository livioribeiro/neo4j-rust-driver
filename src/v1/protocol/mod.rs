pub mod message;

pub const INIT: u8 = 0x01;
pub const RUN: u8 = 0x10;
pub const DISCARD_ALL: u8 = 0x2F;
pub const PULL_ALL: u8 = 0x3F;

pub fn signature(name: &str) -> Option<u8> {
    let sig = match name {
        "INIT" => INIT,
        "RUN" => RUN,
        "DISCARD_ALL" => DISCARD_ALL,
        "PULL_ALL" => PULL_ALL,
        _ => return None,
    };

    Some(sig)
}

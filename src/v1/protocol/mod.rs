pub mod message;

pub const INIT: u8 = 0x01;

pub fn signature(name: &str) -> Option<u8> {
    let sig = match name {
        "INIT" => INIT,
        _ => return None,
    };

    Some(sig)
}

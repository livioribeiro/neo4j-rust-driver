use super::super::marker as M;

pub fn is_tiny_int_pos(b: u8) -> bool { b >> 7 == 0x00 }
pub fn is_tiny_int_neg(b: u8) -> bool { b >> 4 == M::TINY_INT_NEG_NIBBLE >> 4 }
pub fn is_tiny_int(b: u8) -> bool { is_tiny_int_pos(b) || is_tiny_int_neg(b) }

pub fn read_tiny_int(int: u8) -> i8 {
    if is_tiny_int_pos(int) { int as i8 }
    else { (int | 0b1111_0000) as i8 }
}

pub fn is_tiny_string(b: u8) -> bool { b >> 4 == M::TINY_STRING_NIBBLE >> 4 }
pub fn is_tiny_list(b: u8) -> bool { b >> 4 == M::TINY_LIST_NIBBLE >> 4 }
pub fn is_tiny_map(b: u8) -> bool { b >> 4 == M::TINY_MAP_NIBBLE >> 4 }
pub fn is_tiny_structure(b: u8) -> bool { b >> 4 == M::TINY_STRUCT_NIBBLE >> 4 }

pub fn is_int8_or_lesser(b: u8) -> bool {
    b == M::INT_8 || is_tiny_int(b)
}

pub fn is_int16_or_lesser(b: u8) -> bool {
    b == M::INT_16 || is_int8_or_lesser(b)
}

pub fn is_int32_or_lesser(b: u8) -> bool {
    b == M::INT_32 || is_int16_or_lesser(b)
}

pub fn is_int64_or_lesser(b: u8) -> bool {
    b == M::INT_64 || is_int32_or_lesser(b)
}

// pub fn is_list(b: u8) -> bool {
//     is_tiny_list(b) || b == M::LIST_8
//         || b == M::LIST_16 || b == M::LIST_32
// }
//
// pub fn is_map(b: u8) -> bool {
//     is_tiny_map(b) || b == M::MAP_8
//         || b == M::MAP_16 || b == M::MAP_32
// }
//
// pub fn is_structure(b: u8) -> bool {
//     is_tiny_structure(b) || b == M::STRUCT_8 || b == M::STRUCT_16
// }
//
// pub fn is_string(b: u8) -> bool {
//     is_tiny_string(b) || b == M::STRING_8
//         || b == M::STRING_16 || b == M::STRING_32
// }

pub fn which(byte: u8) -> Option<&'static str> {
    match byte {
        M::NULL => Some("NULL"),
        M::TRUE => Some("TRUE"),
        M::FALSE => Some("FALSE"),
        _ if is_tiny_int(byte) => Some("TINY_INT"),
        M::INT_8 => Some("INT_8"),
        M::INT_16 => Some("INT_16"),
        M::INT_32 => Some("INT_32"),
        M::INT_64 => Some("INT_64"),
        M::FLOAT => Some("FLOAT"),
        _ if is_tiny_string(byte) => Some("TINY_STRING"),
        M::STRING_8 => Some("STRING_8"),
        M::STRING_16 => Some("STRING_16"),
        M::STRING_32 => Some("STRING_32"),
        _ if is_tiny_list(byte) => Some("TINY_LIST"),
        M::LIST_8 => Some("LIST_8"),
        M::LIST_16 => Some("LIST_16"),
        M::LIST_32 => Some("LIST_32"),
        _ if is_tiny_map(byte) => Some("TINY_MAP"),
        M::MAP_8 => Some("MAP_8"),
        M::MAP_16 => Some("MAP_16"),
        M::MAP_32 => Some("MAP_32"),
        _ if is_tiny_structure(byte) => Some("TINY_STRUCT"),
        M::STRUCT_8 => Some("STRUCT_8"),
        M::STRUCT_16 => Some("STRUCT_16"),
        _ => None
    }
}

macro_rules! marker {
    ($name:ident = $value:expr) => {
        pub const $name: u8 = $value;
    }
}

// Null and Boolean
marker! { NULL = 0xC0 }
marker! { TRUE = 0xC3 }
marker! { FALSE = 0xC2 }

// Integer
marker! { INT_8 = 0xC8 }
marker! { INT_16 = 0xC9 }
marker! { INT_32 = 0xCA }
marker! { INT_64 = 0xCB }

// Suggested integer representations
pub const RANGE_POS_INT_64: (i64, i64) = (2_147_483_648, 9_223_372_036_854_775_807);
pub const RANGE_POS_INT_32: (i64, i64) = (32_768, 2_147_483_647);
pub const RANGE_POS_INT_16: (i64, i64) = (128, 32_767);
pub const RANGE_TINY_INT: (i64, i64) = (-16, 127);
pub const RANGE_NEG_INT_8: (i64, i64) = (-128, -17);
pub const RANGE_NEG_INT_16: (i64, i64) = (-32_768, -129);
pub const RANGE_NEG_INT_32: (i64, i64) = (-2_147_483_648, -32_769);
pub const RANGE_NEG_INT_64: (i64, i64) = (-9_223_372_036_854_775_808, -2_147_483_649);

// Float
marker! { FLOAT = 0xC1 }

// String
pub const TINY_STRING_NIBBLE: u8 = 0b1000;

marker! { STRING_8 = 0xD0 }
marker! { STRING_16 = 0xD1 }
marker! { STRING_32 = 0xD2 }

pub const USE_TINY_STRING: u16 = 15;
pub const USE_STRING_8: u16 = 255;
pub const USE_STRING_16: u16 = 65_535;
pub const USE_STRING_32: u32 = 4_294_967_295;

// List
pub const TINY_LIST_NIBBLE: u8 = 0b1001;

marker! { LIST_8 = 0xD4 }
marker! { LIST_16 = 0xD5 }
marker! { LIST_32 = 0xD6 }

pub const USE_TINY_LIST: u16 = 15;
pub const USE_LIST_8: u16 = 255;
pub const USE_LIST_16: u16 = 65_535;
pub const USE_LIST_32: u32 = 4_294_967_295;

// Map
pub const TINY_MAP_NIBBLE: u8 = 0b1010;

marker! { MAP_8 = 0xD8 }
marker! { MAP_16 = 0xD9 }
marker! { MAP_32 = 0xDA }

pub const USE_TINY_MAP: u16 = 15;
pub const USE_MAP_8: u16 = 255;
pub const USE_MAP_16: u16 = 65_535;
pub const USE_MAP_32: u32 = 4_294_967_295;

// Map
pub const TINY_STRUCT_NIBBLE: u8 = 0b1011;

marker! { STRUCT_8 = 0xD8 }
marker! { STRUCT_16 = 0xD9 }

pub const USE_TINY_STRUCT: u16 = 15;
pub const USE_STRUCT_8: u16 = 255;
pub const USE_STRUCT_16: u16 = 65_535;

// End marker
pub const END: u16 = 0x00_00;

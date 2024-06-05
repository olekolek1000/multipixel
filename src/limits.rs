pub const TOOL_SIZE_MAX: u8 = 8;

pub const ROOM_NAME_LEN_MIN: u8 = 3;
pub const ROOM_NAME_LEN_MAX: u8 = 24;

pub const NICK_NAME_LEN_MIN: u8 = 3;
pub const NICK_NAME_LEN_MAX: u8 = 24;

pub const BOUNDARY_ZOOM_MIN: f32 = 0.45;

pub const CHUNK_SIZE_PX: u32 = 256;
pub const CHUNK_IMAGE_SIZE_BYTES: usize = (CHUNK_SIZE_PX * CHUNK_SIZE_PX * 3) as usize;

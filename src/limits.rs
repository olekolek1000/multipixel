pub const TOOL_SIZE_BRUSH_MAX: u8 = 16;
pub const TOOL_SIZE_SMOOTH_BRUSH_MAX: u8 = 24;
pub const TOOL_SIZE_SQUARE_BRUSH_MAX: u8 = 16;
pub const TOOL_SIZE_BLUR_MAX: u8 = 16;
pub const TOOL_SIZE_SMUDGE_MAX: u8 = 16;
pub const TOOL_SIZE_SPRAY_MAX: u8 = 32;

pub const ROOM_NAME_LEN_MIN: u8 = 3;
pub const ROOM_NAME_LEN_MAX: u8 = 24;

pub const NICK_NAME_LEN_MIN: u8 = 3;
pub const NICK_NAME_LEN_MAX: u8 = 24;

pub const BOUNDARY_ZOOM_MIN: f32 = 0.45;

pub const CHUNK_SIZE_PX: u32 = 256;
pub const CHUNK_IMAGE_SIZE_BYTES: usize = (CHUNK_SIZE_PX * CHUNK_SIZE_PX * 3) as usize;

pub const PREVIEW_SYSTEM_LAYER_COUNT: u8 = 5;

pub const FLOODFILL_MAX_DISTANCE: u32 = 300;

pub const MIN_ZOOM: f32 = 0.45;

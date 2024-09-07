#![allow(dead_code)]

use std::mem::size_of;

use bytes::{BufMut, Bytes, BytesMut};
use glam::IVec2;

use crate::{chunk::ChunkPixel, limits};

pub enum MessageType {
	PlainText = 0,
	Html = 1,
}

type CommandIndex = u16;
const COMMAND_INDEX_SIZE: usize = size_of::<CommandIndex>();
const SESSION_ID_SIZE: usize = size_of::<u16>();

pub enum ServerCmd {
	Message = 1,                 // u8 type, utf-8 text
	YourId = 2,                  // u16 id
	Kick = 3,                    // u16 text size, utf-8 reason
	ChunkImage = 100,            // complex data
	ChunkPixelPack = 101,        // complex data
	ChunkCreate = 110,           // s32 chunkX, s32 chunkY
	ChunkRemove = 111,           // s32 chunkX, s32 chunkY
	PreviewImage = 200,          // s32 previewX, s32 previewY, u8 zoom, u32 data size, binary data
	UserCreate = 1000,           // u16 id, u8 text_size, utf-8 nickname
	UserRemove = 1001,           // u16 id
	UserCursorPos = 1002,        // u16 id, s32 x, s32 y
	ProcessingStatusText = 1100, // u16 text size, utf-8 text
}

#[derive(Clone)]
pub struct Packet {
	pub data: Bytes,
}

fn prepare_packet_raw(cmd: ServerCmd, data: &[u8]) -> Packet {
	let s = [data];
	prepare_packet_slices(cmd, &s)
}

fn prepare_packet_slices(cmd: ServerCmd, slices: &[&[u8]]) -> Packet {
	let mut payload_size: usize = 0;

	for slice in slices {
		payload_size += slice.len();
	}

	let mut buf = BytesMut::with_capacity(size_of::<CommandIndex>() + payload_size);
	buf.put_u16(cmd as CommandIndex);

	for slice in slices {
		buf.put_slice(slice);
	}

	Packet { data: buf.into() }
}

pub fn prepare_packet_kick(message: &str) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);
	buf.put_u16(ServerCmd::Kick as CommandIndex);
	put_string_u16(&mut buf, message);
	Packet { data: buf.into() }
}

pub fn prepare_packet_status_text(message: &str) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);
	buf.put_u16(ServerCmd::ProcessingStatusText as CommandIndex);
	put_string_u16(&mut buf, message);
	Packet { data: buf.into() }
}

// Put string with u8 size at the start. The output is truncated if it exceeds intended size.
fn put_string_u8(buf: &mut BytesMut, string: &str) {
	let mut bytes = string.as_bytes();
	if bytes.len() > u8::MAX.into() {
		bytes = &bytes[0..u8::MAX.into()]; // Truncate if required
	}
	buf.put_u8(bytes.len() as u8);
	buf.put_slice(bytes);
}

// Put string with u16 size at the start. The output is truncated if it exceeds intended size.
fn put_string_u16(buf: &mut BytesMut, string: &str) {
	let mut bytes = string.as_bytes();
	if bytes.len() > u16::MAX.into() {
		bytes = &bytes[0..u16::MAX.into()]; // Truncate if required
	}
	buf.put_u16(bytes.len() as u16);
	buf.put_slice(bytes);
}

pub fn prepare_packet_your_id(session_id: u32) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE + SESSION_ID_SIZE);

	buf.put_u16(ServerCmd::YourId as CommandIndex);
	buf.put_u16(session_id as u16);

	Packet { data: buf.into() }
}

pub fn prepare_packet_user_cursor_pos(session_id: u32, x: i32, y: i32) -> Packet {
	let mut buf =
		BytesMut::with_capacity(COMMAND_INDEX_SIZE + SESSION_ID_SIZE + size_of::<i32>() * 2);

	buf.put_u16(ServerCmd::UserCursorPos as CommandIndex);
	buf.put_u16(session_id as u16);
	buf.put_i32(x);
	buf.put_i32(y);

	Packet { data: buf.into() }
}

pub fn prepare_packet_user_create(session_id: u32, nickname: &str) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

	buf.put_u16(ServerCmd::UserCreate as CommandIndex);
	buf.put_u16(session_id as u16);
	put_string_u8(&mut buf, nickname);

	Packet { data: buf.into() }
}

pub fn prepare_packet_user_remove(session_id: u32) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

	buf.put_u16(ServerCmd::UserRemove as CommandIndex);
	buf.put_u16(session_id as u16);

	Packet { data: buf.into() }
}

pub fn prepare_packet_chunk_create(chunk_pos: IVec2) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE + size_of::<i32>() * 2);

	buf.put_u16(ServerCmd::ChunkCreate as CommandIndex);
	buf.put_i32(chunk_pos.x);
	buf.put_i32(chunk_pos.y);

	Packet { data: buf.into() }
}

pub fn prepare_packet_preview_image(preview_pos: &IVec2, zoom: u8, data: &[u8]) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

	buf.put_u16(ServerCmd::PreviewImage as CommandIndex);
	buf.put_i32(preview_pos.x);
	buf.put_i32(preview_pos.y);
	buf.put_u8(zoom);
	buf.put_u32(data.len() as u32);
	buf.put_slice(data);

	Packet { data: buf.into() }
}

pub fn prepare_packet_chunk_remove(chunk_pos: IVec2) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE + size_of::<i32>() * 2);

	buf.put_u16(ServerCmd::ChunkRemove as CommandIndex);
	buf.put_i32(chunk_pos.x);
	buf.put_i32(chunk_pos.y);

	Packet { data: buf.into() }
}

pub fn prepare_packet_chunk_image(chunk_pos: IVec2, compressed_chunk_data: &[u8]) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

	buf.put_u16(ServerCmd::ChunkImage as CommandIndex);
	buf.put_i32(chunk_pos.x);
	buf.put_i32(chunk_pos.y);
	buf.put_u32(limits::CHUNK_IMAGE_SIZE_BYTES as u32);
	buf.put_slice(compressed_chunk_data);

	Packet { data: buf.into() }
}

pub fn prepare_packet_message(message_type: MessageType, message: &str) -> Packet {
	let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

	buf.put_u8(message_type as u8);
	buf.put_u16(ServerCmd::Message as CommandIndex);
	put_string_u16(&mut buf, message);

	Packet { data: buf.into() }
}

pub fn prepare_packet_pixel_pack(
	xyrgb_lz4_packed: &[u8],
	chunk_x: i32,
	chunk_y: i32,
	pixel_count: u32,
	raw_size: u32,
) -> Packet {
	let mut buf = BytesMut::new();
	buf.put_u16(ServerCmd::ChunkPixelPack as CommandIndex);
	buf.put_i32(chunk_x);
	buf.put_i32(chunk_y);
	buf.put_u32(pixel_count);
	buf.put_u32(raw_size);
	buf.put_slice(xyrgb_lz4_packed);
	Packet { data: buf.into() }
}

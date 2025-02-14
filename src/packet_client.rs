#![allow(dead_code)]

use binary_reader::BinaryReader;
use num_enum::TryFromPrimitive;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ToolType {
	Brush = 0,
	Fill = 1,
	Spray = 2,
	Blur = 3,
	Smudge = 4,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u16)]
pub enum ClientCmd {
	Message = 1,  // u16 text_size, utf-8 text
	Announce = 2, // u8 room_name_size, utf-8 room_name, u8 nickname_size, utf-8 nickname
	Ping = 4,
	CursorPos = 100, // s32 x, s32 y
	CursorDown = 101,
	CursorUp = 102,
	Boundary = 103,
	ChunksReceived = 104,
	PreviewRequest = 105, // s32 previewX, s32 previewY, u8 zoom
	ToolType = 200,       // u8 type
	ToolColor = 201,      // u8 red, u8 green, u8 blue
	ToolSize = 202,       // u8 size,
	ToolFlow = 203,       // u32 flow
	Undo = 300,
}

pub fn read_string_u8(reader: &mut BinaryReader) -> anyhow::Result<String> {
	let str_size = reader.read_u8()?;
	let str_data = reader.read_bytes(str_size.into())?;
	Ok(String::from(std::str::from_utf8(str_data)?))
}

pub fn read_string_u16(reader: &mut BinaryReader) -> anyhow::Result<String> {
	let str_size = reader.read_u16()?;
	let str_data = reader.read_bytes(str_size.into())?;
	Ok(String::from(std::str::from_utf8(str_data)?))
}

pub struct PacketAnnounce {
	pub room_name: String,
	pub nick_name: String,
}

impl PacketAnnounce {
	pub fn read(reader: &mut BinaryReader) -> anyhow::Result<Self> {
		let room_name_raw = read_string_u8(reader)?;
		let room_name = room_name_raw.to_lowercase();

		Ok(Self {
			room_name,
			nick_name: read_string_u8(reader)?,
		})
	}
}

#[derive(Default, Clone, PartialEq)]
pub struct PacketCursorPos {
	pub x: i32,
	pub y: i32,
}

impl PacketCursorPos {
	pub fn read(reader: &mut BinaryReader) -> Result<Self, std::io::Error> {
		Ok(Self {
			x: reader.read_i32()?,
			y: reader.read_i32()?,
		})
	}
}

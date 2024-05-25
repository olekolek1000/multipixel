#![allow(dead_code)]

use std::mem::size_of;

use bytes::{BufMut, Bytes, BytesMut};
use glam::IVec2;

use crate::session::{Session, SessionID};
pub enum ToolType {
    Brush = 0,
    Fill = 1,
}

pub enum MessageType {
    PlainText = 0,
    Html = 1,
}

type CommandIndex = u16;
const COMMAND_INDEX_SIZE: usize = size_of::<CommandIndex>();

pub enum ClientCmd {
    Message = 1,  // utf-8 text
    Announce = 2, // u8 room_name_size, utf-8 room_name, u8 nickname_size, utf-8 nickname
    Ping = 4,
    CursorPos = 100, // s32 x, s32 y
    CursorDown = 101,
    CursorUp = 102,
    Boundary = 103,
    ChunksReceived = 104,
    PreviewRequest = 105, // s32 previewX, s32 previewY, u8 zoom
    ToolSize = 200,       // u8 size
    ToolColor = 201,      // u8 red, u8 green, u8 blue
    ToolType = 202,       // u8 type
    Undo = 203,
}

pub enum ServerCmd {
    Message = 1,                 // u8 type, utf-8 text
    YourId = 2,                  // u16 id
    Kick = 3,                    // utf-8 reason
    ChunkImage = 100,            // complex data
    ChunkPixelPack = 101,        // complex data
    ChunkCreate = 110,           // s32 chunkX, s32 chunkY
    ChunkRemove = 111,           // s32 chunkX, s32 chunkY
    PreviewImage = 200,          // s32 previewX, s32 previewY, u8 zoom, complex data
    UserCreate = 1000,           // u16 id, utf-8 nickname
    UserRemove = 1001,           // u16 id
    UserCursorPos = 1002,        // u16 id, s32 x, s32 y
    ProcessingStatusText = 1100, // utf-8 text
}

struct Packet {
    data: Bytes,
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

fn prepare_packet_user_cursor_pos(session: &Session, x: i32, y: i32) -> Packet {
    let mut buf =
        BytesMut::with_capacity(COMMAND_INDEX_SIZE + size_of::<SessionID>() + size_of::<i32>() * 2);

    buf.put_u16(ServerCmd::UserCursorPos as CommandIndex);
    buf.put_u16(session.id.0);
    buf.put_i32(x);
    buf.put_i32(y);

    Packet { data: buf.into() }
}

fn prepare_packet_user_create(session: &Session) -> Packet {
    let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

    buf.put_u16(ServerCmd::UserCreate as CommandIndex);
    buf.put_u16(session.id.0);
    put_string_u8(&mut buf, session.nickname.as_str());

    Packet { data: buf.into() }
}

fn prepare_packet_user_remove(session: &Session) -> Packet {
    let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

    buf.put_u16(ServerCmd::UserRemove as CommandIndex);
    buf.put_u16(session.id.0);

    Packet { data: buf.into() }
}

fn prepare_packet_chunk_create(chunk_pos: IVec2) -> Packet {
    let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE + size_of::<i32>() * 2);

    buf.put_u16(ServerCmd::ChunkCreate as CommandIndex);
    buf.put_i32(chunk_pos.x);
    buf.put_i32(chunk_pos.y);

    Packet { data: buf.into() }
}

fn prepare_packet_chunk_remove(chunk_pos: IVec2) -> Packet {
    let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE + size_of::<i32>() * 2);

    buf.put_u16(ServerCmd::ChunkRemove as CommandIndex);
    buf.put_i32(chunk_pos.x);
    buf.put_i32(chunk_pos.y);

    Packet { data: buf.into() }
}

fn prepare_packet_message(message_type: MessageType, message: &str) -> Packet {
    let mut buf = BytesMut::with_capacity(COMMAND_INDEX_SIZE);

    buf.put_u8(message_type as u8);
    buf.put_u16(ServerCmd::Message as CommandIndex);
    put_string_u16(&mut buf, message);

    Packet { data: buf.into() }
}

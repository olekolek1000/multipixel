use std::{
	collections::VecDeque,
	sync::{Arc, Weak},
};

use glam::{IVec2, UVec2};
use tokio::sync::Mutex;

use crate::{
	gen_id,
	packet_client::Color,
	session::{SessionHandle, SessionInstance},
};

struct ChunkPixel {
	pub pos: UVec2,
	pub color: Color,
}

pub struct ChunkInstance {
	new_chunk: bool,
	position: IVec2,
	modified: bool,
	queued_pixels_to_send: VecDeque<ChunkPixel>,
	raw_image_data: Option<Vec<u8>>,
	compressed_image_data: Option<Vec<u8>>,
	send_chunk_data_instead_of_pixels: bool,
	linked_sessions: Vec<SessionHandle>,
}

impl ChunkInstance {
	pub fn new(position: IVec2, compressed_image_data: Option<Vec<u8>>) -> Self {
		Self {
			new_chunk: compressed_image_data.is_some(),
			modified: false,
			queued_pixels_to_send: Default::default(),
			position,
			compressed_image_data,
			raw_image_data: None,
			send_chunk_data_instead_of_pixels: false,
			linked_sessions: Default::default(),
		}
	}

	pub fn link_session(&mut self, session: &SessionInstance) {
		todo!();
	}
}

pub type ChunkInstanceMutex = Arc<Mutex<ChunkInstance>>;
pub type ChunkInstanceWeak = Weak<Mutex<ChunkInstance>>;
gen_id!(ChunkVec, ChunkInstanceMutex, ChunkCell, ChunkHandle);

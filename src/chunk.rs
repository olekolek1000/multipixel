use std::{
	collections::VecDeque,
	sync::{Arc, OnceLock, Weak},
};

use glam::{IVec2, UVec2};
use tokio::sync::Mutex;

use crate::{
	compression, gen_id, limits, packet_server,
	pixel::Color,
	session::{SessionHandle, SessionInstance, SessionInstanceMutex},
};

pub struct ChunkPixel {
	pub pos: UVec2,
	pub color: Color,
}

struct LinkedSession {
	sesison: SessionInstanceMutex,
	handle: SessionHandle,
}

pub struct ChunkInstance {
	new_chunk: bool,
	position: IVec2,
	modified: bool,
	queued_pixels_to_send: VecDeque<ChunkPixel>,
	raw_image_data: Option<Vec<u8>>,
	compressed_image_data: Option<Arc<Vec<u8>>>,
	send_chunk_data_instead_of_pixels: bool,
	linked_sessions: Vec<LinkedSession>,
}

// Returns LZ4-compressed empty, white chunk. Generates once.
fn get_empty_chunk() -> &'static std::sync::Mutex<Arc<Vec<u8>>> {
	static CHUNK_DATA: OnceLock<std::sync::Mutex<Arc<Vec<u8>>>> = OnceLock::new();
	CHUNK_DATA.get_or_init(|| {
		let mut stub_img: Vec<u8> = Vec::new();
		stub_img.resize(limits::CHUNK_IMAGE_SIZE_BYTES as usize, 255);
		std::sync::Mutex::new(Arc::new(compression::compress_lz4(&stub_img)))
	})
}

impl ChunkInstance {
	pub fn new(position: IVec2, compressed_image_data: Option<Vec<u8>>) -> Self {
		Self {
			new_chunk: compressed_image_data.is_some(),
			modified: false,
			queued_pixels_to_send: Default::default(),
			position,
			compressed_image_data: if let Some(data) = compressed_image_data {
				Some(Arc::new(data))
			} else {
				None
			},
			raw_image_data: None,
			send_chunk_data_instead_of_pixels: false,
			linked_sessions: Default::default(),
		}
	}

	fn allocate_image(&mut self) {
		if self.raw_image_data.is_some() {
			return; // Nothing to do, already allocated
		}

		if let Some(compressed) = &self.compressed_image_data {
			// Decode compressed data
			if let Some(raw) = compression::decompress_lz4(compressed, limits::CHUNK_IMAGE_SIZE_BYTES) {
				self.raw_image_data = Some(raw);
				return;
			}
		}

		// Failed to load image, allocate white chunk
		let mut data: Vec<u8> = Vec::new();
		data.resize(limits::CHUNK_IMAGE_SIZE_BYTES, 255); // White color
		self.raw_image_data = Some(data);
	}

	fn set_modified(&mut self, modified: bool) {
		self.modified = modified;
		if modified {
			// Invalidate compressed data
			self.compressed_image_data = None;
		}
	}

	fn encode_chunk_data(&mut self, clear_modified: bool) -> Arc<Vec<u8>> {
		let compressed = if self.new_chunk {
			// Return compressed empty chunk
			return get_empty_chunk().lock().unwrap().clone();
		} else {
			self.allocate_image();
			let raw_image = self.raw_image_data.as_ref().unwrap();
			let data = Arc::new(compression::compress_lz4(raw_image));
			self.compressed_image_data = Some(data.clone());
			data
		};

		if clear_modified {
			self.set_modified(false);
			self.raw_image_data = None;

			todo!()
			/*
			auto *preview_system = chunk_system->room->getPreviewSystem();

			Int2 upper_pos;
			upper_pos.x = position.x >= 0 ? position.x / 2 : (position.x - 1) / 2;
			upper_pos.y = position.y >= 0 ? position.y / 2 : (position.y - 1) / 2;
			preview_system->addToQueueFront(upper_pos);
				 */
		}

		compressed
	}

	pub fn send_chunk_data_to_session(&mut self, session: &mut SessionInstance) {
		let compressed_data = if let Some(compressed) = &self.compressed_image_data {
			compressed.clone()
		} else {
			self.encode_chunk_data(false)
		};

		session.queue_send_packet(packet_server::prepare_packet_chunk_image(
			self.position,
			&compressed_data,
		))
	}

	pub fn is_linked_sessions_empty(&self) -> bool {
		self.linked_sessions.is_empty()
	}

	pub fn link_session(&mut self, handle: &SessionHandle, session: &SessionInstanceMutex) {
		for s in &self.linked_sessions {
			if s.handle == *handle {
				log::error!("Session is already linked!");
			}
		}

		self.linked_sessions.push(LinkedSession {
			handle: *handle,
			sesison: session.clone(),
		});
	}

	pub fn unlink_session(&mut self, handle: &SessionHandle) {
		let mut to_remove_idx: Option<usize> = None;
		for (idx, session) in self.linked_sessions.iter().enumerate() {
			if session.handle == *handle {
				to_remove_idx = Some(idx);
				break;
			}
		}

		if let Some(idx) = to_remove_idx {
			self.linked_sessions.remove(idx);
		} else {
			log::warn!("Cannot unlink non-existent session");
		}

		if self.linked_sessions.is_empty() {
			//chunk_system->markGarbageCollect();
			todo!();
		}
	}
}

pub type ChunkInstanceMutex = Arc<Mutex<ChunkInstance>>;
pub type ChunkInstanceWeak = Weak<Mutex<ChunkInstance>>;
gen_id!(ChunkVec, ChunkInstanceMutex, ChunkCell, ChunkHandle);

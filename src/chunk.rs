use std::sync::{Arc, OnceLock, Weak};

use bytes::{BufMut, BytesMut};
use glam::{IVec2, UVec2};
use std::sync::Mutex as SyncMutex;
use tokio::sync::Mutex;

use crate::{
	compression::{self, compress_lz4},
	event_queue::EventQueue,
	gen_id,
	limits::{self, CHUNK_SIZE_PX},
	packet_server::{self, prepare_packet_pixel_pack},
	pixel::Color,
	preview_system::PreviewSystemQueuedChunks,
	session::{SessionHandle, SessionInstanceWeak},
	signal::Signal,
};

#[derive(Clone)]
pub struct ChunkPixel {
	pub pos: UVec2,
	pub color: Color,
}

pub struct LinkedSession {
	_session: SessionInstanceWeak,
	queue_send: EventQueue<packet_server::Packet>,
	handle: SessionHandle,
}

#[derive(Clone)]
pub struct ChunkInstanceRefs {
	pub modified: Arc<SyncMutex<bool>>,
	pub linked_sessions: Arc<SyncMutex<Vec<LinkedSession>>>,
}

pub struct ChunkInstance {
	new_chunk: bool,
	pub position: IVec2,
	refs: ChunkInstanceRefs,
	pub raw_image_data: Option<Vec<u8>>,
	compressed_image_data: Option<Arc<Vec<u8>>>,
	preview_system_queued_chunks: PreviewSystemQueuedChunks,
	signal_garbage_collect: Signal,
}

// Returns LZ4-compressed empty, white chunk. Generates once.
fn get_empty_chunk() -> &'static std::sync::Mutex<Arc<Vec<u8>>> {
	static CHUNK_DATA: OnceLock<std::sync::Mutex<Arc<Vec<u8>>>> = OnceLock::new();
	CHUNK_DATA.get_or_init(|| {
		let mut stub_img: Vec<u8> = Vec::new();
		stub_img.resize(limits::CHUNK_IMAGE_SIZE_BYTES, 255);
		std::sync::Mutex::new(Arc::new(compression::compress_lz4(&stub_img)))
	})
}

impl ChunkInstance {
	pub fn new(
		position: IVec2,
		refs: ChunkInstanceRefs,
		preview_system_queued_chunks: PreviewSystemQueuedChunks,
		signal_garbage_collect: Signal,
		compressed_image_data: Option<Vec<u8>>,
	) -> Self {
		Self {
			new_chunk: compressed_image_data.is_some(),
			refs,
			position,
			preview_system_queued_chunks,
			signal_garbage_collect,
			compressed_image_data: compressed_image_data.map(Arc::new),
			raw_image_data: None,
		}
	}

	pub fn allocate_image(&mut self) {
		if self.raw_image_data.is_some() {
			return; // Nothing to do, already allocated
		}

		self.new_chunk = false;

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
		*self.refs.modified.lock().unwrap() = modified;
		if modified {
			// Invalidate compressed data
			self.compressed_image_data = None;
		}
	}

	pub fn encode_chunk_data(&mut self, clear_modified: bool) -> Arc<Vec<u8>> {
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

			let upper_pos = IVec2::new(
				if self.position.x >= 0 {
					self.position.x / 2
				} else {
					(self.position.x - 1) / 2
				},
				if self.position.y >= 0 {
					self.position.y / 2
				} else {
					(self.position.y - 1) / 2
				},
			);
			self.preview_system_queued_chunks.send(upper_pos);
		}

		compressed
	}

	/// Use allocate_image() first before calling this function.
	pub fn get_pixel(&self, chunk_pixel_pos: UVec2) -> Color {
		debug_assert!(self.raw_image_data.is_some());

		let offset = (chunk_pixel_pos.y * CHUNK_SIZE_PX * 3 + chunk_pixel_pos.x * 3) as usize;

		if let Some(data) = &self.raw_image_data {
			return Color {
				r: data[offset],
				g: data[offset + 1],
				b: data[offset + 2],
			};
		}

		unreachable!();
	}

	pub fn set_pixels(&mut self, pixels: &[ChunkPixel], only_send: bool, send_whole_chunk: bool) {
		debug_assert!(self.raw_image_data.is_some());

		if send_whole_chunk {
			for pixel in pixels {
				// Update pixel
				let offset = (pixel.pos.y * CHUNK_SIZE_PX * 3 + pixel.pos.x * 3) as usize;
				if let Some(data) = &mut self.raw_image_data {
					data[offset] = pixel.color.r;
					data[offset + 1] = pixel.color.g;
					data[offset + 2] = pixel.color.b;
				} else {
					unreachable!()
				}
			}

			self.set_modified(true);

			let session_send_queues: Vec<_> = self
				.refs
				.linked_sessions
				.lock()
				.unwrap()
				.iter()
				.map(|session| session.queue_send.clone())
				.collect();
			for send_queue in session_send_queues {
				self.send_chunk_data_to_session(send_queue);
			}
		} else {
			// Prepare pixel_pack packet
			let mut buf = BytesMut::new();
			let mut pixel_count = 0;

			for pixel in pixels {
				if !only_send {
					let color = self.get_pixel(pixel.pos);
					if pixel.color == color {
						// Pixel not changed, skip
						continue;
					}

					// Update pixel
					let offset = (pixel.pos.y * CHUNK_SIZE_PX * 3 + pixel.pos.x * 3) as usize;
					if let Some(data) = &mut self.raw_image_data {
						data[offset] = pixel.color.r;
						data[offset + 1] = pixel.color.g;
						data[offset + 2] = pixel.color.b;
					} else {
						unreachable!()
					}
				}

				// Prepare pixel data
				buf.put_u8(pixel.pos.x as u8);
				buf.put_u8(pixel.pos.y as u8);
				buf.put_u8(pixel.color.r);
				buf.put_u8(pixel.color.g);
				buf.put_u8(pixel.color.b);
				pixel_count += 1;
			}

			if pixel_count == 0 {
				// Nothing modified
				return;
			}

			self.set_modified(true);

			// LZ4-compressed (xyrgb,xyrgb,xyrgb...) data
			let compressed_buf = compress_lz4(&buf);

			let packet = prepare_packet_pixel_pack(
				&compressed_buf,
				self.position.x,
				self.position.y,
				pixel_count,
				buf.len() as u32,
			);

			for session in self.refs.linked_sessions.lock().unwrap().iter() {
				session.queue_send.send(packet.clone());
			}
		}
	}

	pub fn send_chunk_data_to_session(
		&mut self,
		session_queue_send: EventQueue<packet_server::Packet>,
	) {
		let compressed_data = if let Some(compressed) = &self.compressed_image_data {
			compressed.clone()
		} else {
			self.encode_chunk_data(false)
		};

		session_queue_send.send(packet_server::prepare_packet_chunk_image(
			self.position,
			&compressed_data,
		));
	}

	pub fn link_session(
		&mut self,
		handle: &SessionHandle,
		session: SessionInstanceWeak,
		session_queue_send: EventQueue<packet_server::Packet>,
	) {
		let mut linked_sessions = self.refs.linked_sessions.lock().unwrap();

		for s in linked_sessions.iter() {
			if s.handle == *handle {
				log::error!("Session is already linked!");
			}
		}

		linked_sessions.push(LinkedSession {
			handle: *handle,
			_session: session.clone(),
			queue_send: session_queue_send,
		});
	}

	pub fn unlink_session(&mut self, handle: &SessionHandle) {
		let mut to_remove_idx: Option<usize> = None;

		let mut linked_sessions = self.refs.linked_sessions.lock().unwrap();

		for (idx, session) in linked_sessions.iter().enumerate() {
			if session.handle == *handle {
				to_remove_idx = Some(idx);
				break;
			}
		}

		if let Some(idx) = to_remove_idx {
			linked_sessions.remove(idx);
		} else {
			log::warn!("Cannot unlink non-existent session");
		}

		if linked_sessions.is_empty() {
			self.signal_garbage_collect.notify();
		}
	}
}

pub type ChunkInstanceMutex = Arc<Mutex<ChunkInstance>>;
pub type ChunkInstanceWeak = Weak<Mutex<ChunkInstance>>;
gen_id!(ChunkVec, ChunkInstanceMutex, ChunkCell, ChunkHandle);

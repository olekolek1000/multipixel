use std::sync::{Arc, OnceLock, Weak};

use bytes::{BufMut, BytesMut};
use glam::{IVec2, U8Vec2};
use std::sync::Mutex as SyncMutex;
use tokio::sync::Mutex;

use crate::{
	compression::{self, compress_lz4},
	event_queue::EventQueue,
	gen_id,
	limits::{self, CHUNK_SIZE_PX},
	packet_server::{self, prepare_packet_pixel_pack},
	pixel::{ColorRGB, ColorRGBA},
	preview_system::PreviewSystemQueuedChunks,
	session::{SessionHandle, SessionInstanceWeak},
	signal::Signal,
};

use super::{
	compositor::{self, Compositor},
	layer::{LayerRGB, RGBData},
};

#[derive(Clone)]
pub struct ChunkPixelRGB {
	pub pos: U8Vec2,
	pub color: ColorRGB,
}

#[derive(Clone)]
pub struct ChunkPixelRGBA {
	pub pos: U8Vec2,
	pub color: ColorRGBA,
}

pub struct LinkedSession {
	_session: SessionInstanceWeak,
	queue_send: EventQueue<packet_server::Packet>,
	handle: SessionHandle,
}

#[derive(Clone)]
pub struct ChunkInstanceRefs {
	pub main_modified: Arc<SyncMutex<bool>>,
	pub linked_sessions: Arc<SyncMutex<Vec<LinkedSession>>>,
}

pub struct ChunkInstance {
	new_chunk: bool,
	pub position: IVec2,
	refs: ChunkInstanceRefs,

	main_layer: LayerRGB,
	pub compositor: compositor::Compositor,

	compressed_image_data: Option<Arc<Vec<u8>>>,
	preview_system_queued_chunks: PreviewSystemQueuedChunks,
	signal_garbage_collect: Signal,
}

// Returns LZ4-compressed empty, white chunk. Generates once.
fn get_empty_chunk_rgb() -> &'static std::sync::Mutex<Arc<Vec<u8>>> {
	static CHUNK_DATA: OnceLock<std::sync::Mutex<Arc<Vec<u8>>>> = OnceLock::new();
	CHUNK_DATA.get_or_init(|| {
		let mut stub_img: Vec<u8> = Vec::new();
		stub_img.resize(limits::CHUNK_IMAGE_SIZE_BYTES_RGB, 255);
		std::sync::Mutex::new(Arc::new(compression::compress_lz4(&stub_img)))
	})
}

fn gen_pixel_pack(buf: &mut BytesMut, pixels: &[ChunkPixelRGB]) {
	// Prepare pixel data
	for pixel in pixels {
		buf.put_u8(pixel.pos.x);
		buf.put_u8(pixel.pos.y);
		buf.put_u8(pixel.color.r);
		buf.put_u8(pixel.color.g);
		buf.put_u8(pixel.color.b);
	}
}

fn gen_packet_pixel_pack(chunk_pos: IVec2, pixels: &[ChunkPixelRGB]) -> packet_server::Packet {
	let mut buf = BytesMut::new();

	gen_pixel_pack(&mut buf, pixels);

	prepare_packet_pixel_pack(
		&compress_lz4(&buf),
		chunk_pos.x,
		chunk_pos.y,
		pixels.len() as u32,
		buf.len() as u32,
	)
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
			main_layer: LayerRGB::new(),
			compositor: compositor::Compositor::new(),
		}
	}

	pub fn allocate_image(&mut self) {
		if self.main_layer.read().is_some() {
			return; // Nothing to do, already allocated
		}

		self.new_chunk = false;

		if let Some(compressed) = &self.compressed_image_data {
			// Decode compressed data
			if let Some(raw) = compression::decompress_lz4(compressed, limits::CHUNK_IMAGE_SIZE_BYTES_RGB)
			{
				self.main_layer.apply(RGBData(raw));
				return;
			}
		}

		// Failed to load image, allocate white chunk
		self.main_layer.alloc_white();
	}

	fn set_main_modified(&mut self, modified: bool) {
		*self.refs.main_modified.lock().unwrap() = modified;
		if modified {
			// Invalidate compressed data
			self.compressed_image_data = None;
		}
	}

	pub fn get_pixel_main(&self, chunk_pixel_pos: U8Vec2) -> ColorRGB {
		self.main_layer.get_pixel(chunk_pixel_pos)
	}

	pub fn get_layer_main(&self) -> &LayerRGB {
		&self.main_layer
	}

	pub fn replace_layer_main(&mut self, layer: LayerRGB) {
		self.main_layer = layer;
		self.set_main_modified(true);
		self.send_chunk_data_to_all();
	}

	pub fn encode_composited_chunk_data(&mut self, session_handle: &SessionHandle) -> Vec<u8> {
		self.allocate_image();

		let layers = self
			.compositor
			.construct_layers_from_session(session_handle);

		compression::compress_lz4(
			&compositor::Compositor::composite(&self.main_layer, &layers)
				.read_unchecked()
				.0,
		)
	}

	pub fn encode_chunk_data(&mut self, clear_modified: bool) -> Arc<Vec<u8>> {
		let compressed = if self.new_chunk {
			// Return compressed empty chunk
			return get_empty_chunk_rgb().lock().unwrap().clone();
		} else {
			self.allocate_image();
			let raw_image = self.main_layer.read_unchecked();
			let data = Arc::new(compression::compress_lz4(&raw_image.0));
			self.compressed_image_data = Some(data.clone());
			data
		};

		if clear_modified {
			self.set_main_modified(false);
			self.main_layer.free();

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

	fn set_pixels_internal(&mut self, pixels: &[ChunkPixelRGB]) {
		let layer_data = self.main_layer.read_unchecked_mut();
		for pixel in pixels {
			// Update pixel
			let offset = (pixel.pos.y as u32 * CHUNK_SIZE_PX * 3 + pixel.pos.x as u32 * 3) as usize;
			(layer_data.0)[offset] = pixel.color.r;
			(layer_data.0)[offset + 1] = pixel.color.g;
			(layer_data.0)[offset + 2] = pixel.color.b;
		}
	}

	fn set_pixels_whole_chunk(&mut self, pixels: &[ChunkPixelRGB]) {
		self.set_pixels_internal(pixels);
		self.set_main_modified(true);
		self.send_chunk_data_to_all();
	}

	fn set_pixels_pack(&mut self, pixels: &[ChunkPixelRGB]) {
		let mut modified_pixels = Vec::<ChunkPixelRGB>::new();

		for pixel in pixels {
			let color = self.main_layer.get_pixel(pixel.pos);
			if pixel.color == color {
				continue; // Pixel not changed, skip
			}

			let layer_data = self.main_layer.read_unchecked_mut();

			// Update pixel
			let offset = (pixel.pos.y as u32 * CHUNK_SIZE_PX * 3 + pixel.pos.x as u32 * 3) as usize;
			(layer_data.0)[offset] = pixel.color.r;
			(layer_data.0)[offset + 1] = pixel.color.g;
			(layer_data.0)[offset + 2] = pixel.color.b;

			modified_pixels.push(pixel.clone());
		}

		// Nothing modified, do nothing.
		if modified_pixels.is_empty() {
			return;
		}

		// We've modified main layer pixel data. Mark it as modified to save it to the database later.
		self.set_main_modified(true);

		let mut packet_pixel_pack_plain: Option<packet_server::Packet> = None;

		for session in self.refs.linked_sessions.lock().unwrap().iter() {
			if !self.compositor.has_session_composition(&session.handle) {
				// mark that we will generate plain pixel pack for all users
				// not using composition layers
				packet_pixel_pack_plain = Some(gen_packet_pixel_pack(self.position, &modified_pixels));
				break;
			}
		}

		for session in self.refs.linked_sessions.lock().unwrap().iter() {
			if !self.compositor.has_session_composition(&session.handle) {
				// Send plain pixel data
				session
					.queue_send
					.send(packet_pixel_pack_plain.as_ref().unwrap().clone());
			} else {
				// Send composited pixel pack exclusively for this client
				let layers = self
					.compositor
					.construct_layers_from_session(&session.handle);
				let mut composited_pixels = Vec::<ChunkPixelRGB>::with_capacity(modified_pixels.len());

				for pixel in &modified_pixels {
					let rgb = Compositor::calc_pixel(&self.main_layer, &layers, pixel.pos);
					composited_pixels.push(ChunkPixelRGB {
						color: rgb,
						pos: pixel.pos,
					})
				}

				let packet = gen_packet_pixel_pack(self.position, &composited_pixels);
				session.queue_send.send(packet);
			}
		}
	}

	pub fn set_pixels(&mut self, pixels: &[ChunkPixelRGB], send_whole_chunk: bool) {
		debug_assert!(self.main_layer.read().is_some());

		if send_whole_chunk {
			self.set_pixels_whole_chunk(pixels);
		} else {
			self.set_pixels_pack(pixels);
		}
	}

	pub fn send_pixel_updates(&mut self, coords: &[U8Vec2]) {
		debug_assert!(self.main_layer.read().is_some());

		for session in self.refs.linked_sessions.lock().unwrap().iter() {
			let layers = self
				.compositor
				.construct_layers_from_session(&session.handle);
			let mut composited_pixels = Vec::<ChunkPixelRGB>::with_capacity(coords.len());

			for coord in coords {
				let rgb = Compositor::calc_pixel(&self.main_layer, &layers, *coord);
				composited_pixels.push(ChunkPixelRGB {
					color: rgb,
					pos: *coord,
				})
			}

			let packet = gen_packet_pixel_pack(self.position, &composited_pixels);
			session.queue_send.send(packet);
		}
	}

	pub fn send_chunk_data_to_all(&mut self) {
		let session_send_queues: Vec<_> = self
			.refs
			.linked_sessions
			.lock()
			.unwrap()
			.iter()
			.map(|session| (session.handle, session.queue_send.clone()))
			.collect();

		for (session_handle, send_queue) in session_send_queues {
			self.send_chunk_data_to_session(&session_handle, send_queue);
		}
	}

	pub fn send_chunk_data_to_session(
		&mut self,
		session_handle: &SessionHandle,
		session_queue_send: EventQueue<packet_server::Packet>,
	) {
		let composited = self.compositor.has_session_composition(session_handle);

		let compressed_data = match composited {
			false => {
				// read compressed data directly from the memory, no compositing needed
				if let Some(compressed) = &self.compressed_image_data {
					compressed.clone()
				} else {
					self.encode_chunk_data(false)
				}
			}
			true => {
				// composite image data, compress and send it
				Arc::new(self.encode_composited_chunk_data(session_handle))
			}
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

		self.compositor.deref_session(handle);
	}
}

pub type ChunkInstanceMutex = Arc<Mutex<ChunkInstance>>;
pub type ChunkInstanceWeak = Weak<Mutex<ChunkInstance>>;
gen_id!(ChunkVec, ChunkInstanceMutex, ChunkCell, ChunkHandle);

use std::{collections::HashMap, sync::Arc};

use glam::{IVec2, UVec2};
use tokio::sync::Mutex;

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex, ChunkPixel},
	database::Database,
	limits::CHUNK_SIZE_PX,
	pixel::GlobalPixel,
	room::RoomInstance,
};

pub struct ChunkSystem {
	chunks: HashMap<IVec2, ChunkInstanceMutex>,
}

fn modulo(x: i32, n: i32) -> i32 {
	(x % n + n) % n
}

impl ChunkSystem {
	pub fn new() -> Self {
		Self {
			chunks: HashMap::new(),
		}
	}

	pub fn global_pixel_pos_to_chunk_pos(pixel_pos: IVec2) -> IVec2 {
		let mut chunk_pos_x = (pixel_pos.x + (pixel_pos.x < 0) as i32) / CHUNK_SIZE_PX as i32;
		let mut chunk_pos_y = (pixel_pos.y + (pixel_pos.y < 0) as i32) / CHUNK_SIZE_PX as i32;

		if pixel_pos.x < 0 {
			chunk_pos_x -= 1;
		}
		if pixel_pos.y < 0 {
			chunk_pos_y -= 1;
		}

		IVec2::new(chunk_pos_x, chunk_pos_y)
	}

	pub fn global_pixel_pos_to_local_pixel_pos(global_pixel_pos: IVec2) -> UVec2 {
		let x = modulo(global_pixel_pos.x, CHUNK_SIZE_PX as i32);
		let y = modulo(global_pixel_pos.y, CHUNK_SIZE_PX as i32);
		UVec2::new(x as u32, y as u32)
	}

	pub async fn get_chunk(
		&mut self,
		room: &RoomInstance,
		chunk_pos: IVec2,
	) -> anyhow::Result<ChunkInstanceMutex> {
		if let Some(chunk) = self.chunks.get(&chunk_pos) {
			// Return previously loaded chunk
			return Ok(chunk.clone());
		}

		let mut compressed_chunk_data: Option<Vec<u8>> = None;

		// Load chunk pixels from the database
		if let Some(record) = room
			.database
			.client
			.conn(move |conn| Database::chunk_load_data(conn, chunk_pos))
			.await?
		{
			compressed_chunk_data = Some(record.data);
		}

		// Allocate chunk
		let chunk_mtx = Arc::new(Mutex::new(ChunkInstance::new(
			chunk_pos,
			compressed_chunk_data,
		)));

		self.chunks.insert(chunk_pos, chunk_mtx.clone());

		Ok(chunk_mtx)
	}

	pub async fn set_pixels_global(
		&mut self,
		room: &RoomInstance,
		pixels: Vec<GlobalPixel>,
		queued: bool,
	) {
		struct ChunkCacheCell {
			chunk_pos: IVec2,
			chunk: ChunkInstanceMutex,
			queued_pixels: Vec<ChunkPixel>,
		}

		fn fetch_cell<'a>(
			affected_chunks: &'a mut Vec<ChunkCacheCell>,
			chunk_pos: &IVec2,
		) -> Option<&'a mut ChunkCacheCell> {
			affected_chunks
				.iter_mut()
				.find(|cell| cell.chunk_pos == *chunk_pos)
		}

		async fn cache_new_chunk(
			chunk_system: &mut ChunkSystem,
			room: &RoomInstance,
			affected_chunks: &mut Vec<ChunkCacheCell>,
			chunk_pos: &IVec2,
		) {
			if let Ok(chunk) = chunk_system.get_chunk(room, *chunk_pos).await {
				affected_chunks.push(ChunkCacheCell {
					chunk_pos: chunk_pos.clone(),
					chunk,
					queued_pixels: Default::default(),
				});
			}
		}

		let mut affected_chunks: Vec<ChunkCacheCell> = Vec::new();

		// Generate affected chunks list
		for pixel in &pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if fetch_cell(&mut affected_chunks, &chunk_pos).is_none() {
				cache_new_chunk(self, room, &mut affected_chunks, &chunk_pos);
			}
		}

		// Send pixels to chunks
		for pixel in &pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if let Some(cell) = fetch_cell(&mut affected_chunks, &chunk_pos) {
				cell.queued_pixels.push(ChunkPixel {
					color: pixel.color.clone(),
					pos: ChunkSystem::global_pixel_pos_to_local_pixel_pos(pixel.pos),
				});
			} else {
				// Skip pixel, already set
				continue;
			}
		}

		for cell in &affected_chunks {
			if cell.queued_pixels.is_empty() {
				continue;
			}
		}

		todo!("todo!")
	}
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;

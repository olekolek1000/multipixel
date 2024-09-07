use std::{collections::HashMap, sync::Arc};

use glam::{IVec2, UVec2};
use tokio::sync::Mutex;

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex},
	database::Database,
	limits::CHUNK_SIZE_PX,
};

pub struct ChunkSystem {
	chunks: HashMap<IVec2, ChunkInstanceMutex>,
	database: Arc<Mutex<Database>>,
}

fn modulo(x: i32, n: i32) -> i32 {
	(x % n + n) % n
}

impl ChunkSystem {
	pub fn new(database: Arc<Mutex<Database>>) -> Self {
		Self {
			chunks: HashMap::new(),
			database,
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

	pub async fn get_chunk(&mut self, chunk_pos: IVec2) -> anyhow::Result<ChunkInstanceMutex> {
		if let Some(chunk) = self.chunks.get(&chunk_pos) {
			// Return previously loaded chunk
			return Ok(chunk.clone());
		}

		let mut compressed_chunk_data: Option<Vec<u8>> = None;

		// Load chunk pixels from the database
		if let Some(record) = self
			.database
			.lock()
			.await
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
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;

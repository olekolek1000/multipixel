use std::{collections::HashMap, sync::Arc};

use glam::IVec2;
use tokio::sync::{Mutex, MutexGuard};

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex, ChunkInstanceWeak},
	database::Database,
	room::{RoomInstance, RoomInstanceMutex},
	session::SessionInstance,
};

pub struct ChunkSystem {
	chunks: HashMap<IVec2, ChunkInstanceMutex>,
}

impl ChunkSystem {
	pub fn new() -> Self {
		Self {
			chunks: HashMap::new(),
		}
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
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;

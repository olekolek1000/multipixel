use std::sync::Arc;

use glam::IVec2;

use crate::{
	chunk::{ChunkInstanceMutex, ChunkInstanceWeak},
	chunk_system::ChunkSystemMutex,
};

#[derive(Default)]
pub struct ChunkCache {
	pub chunk: ChunkInstanceWeak,
	pub chunk_pos: Option<IVec2>,
}

impl ChunkCache {
	pub async fn get(
		&mut self,
		chunk_system_mtx: &ChunkSystemMutex,
		chunk_pos: IVec2,
	) -> Option<ChunkInstanceMutex> {
		if let Some(cache_chunk_pos) = self.chunk_pos {
			if cache_chunk_pos == chunk_pos {
				return self.chunk.upgrade();
			}
		}

		let mut chunk_system = chunk_system_mtx.lock().await;
		if let Ok(chunk) = chunk_system.get_chunk(chunk_pos).await {
			self.chunk = Arc::downgrade(&chunk);
			self.chunk_pos = Some(chunk_pos);
			return Some(chunk);
		}

		None
	}
}

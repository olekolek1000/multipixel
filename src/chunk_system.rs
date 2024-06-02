use std::sync::Arc;

use glam::IVec2;
use tokio::sync::{Mutex, MutexGuard};

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex},
	session::SessionInstance,
};

pub struct ChunkSystem {}

impl ChunkSystem {
	pub fn new() -> Self {
		Self {}
	}

	pub fn get_chunk_mtx(&self, chunk_pos: IVec2) -> &ChunkInstanceMutex {
		todo!()
	}

	pub async fn get_chunk(&self, chunk_pos: IVec2) -> MutexGuard<ChunkInstance> {
		self.get_chunk_mtx(chunk_pos).lock().await
	}
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;

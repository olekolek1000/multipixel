use std::{
	collections::HashMap,
	sync::{Arc, Weak},
};

use futures_util::{future::Shared, FutureExt};
use glam::{IVec2, UVec2};
use tokio::{
	sync::{Mutex, Notify},
	task::JoinHandle,
};

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex, ChunkInstanceWeak},
	database::Database,
	limits::CHUNK_SIZE_PX,
	time::get_millis,
};

pub struct ChunkSystem {
	chunks: HashMap<IVec2, ChunkInstanceMutex>,
	database: Arc<Mutex<Database>>,
	notify: Arc<Notify>,
	cleaned_up: bool,
	autosave_interval_ms: u32,
	last_autosave_timestamp: u64,
	task_tick: Option<JoinHandle<()>>,
}

fn modulo(x: i32, n: i32) -> i32 {
	(x % n + n) % n
}

impl Drop for ChunkSystem {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
		log::debug!("Chunk system dropped");
	}
}

impl ChunkSystem {
	pub fn new(database: Arc<Mutex<Database>>, autosave_interval_ms: u32) -> Self {
		Self {
			chunks: HashMap::new(),
			database,
			notify: Arc::new(Notify::new()),
			cleaned_up: false,
			autosave_interval_ms,
			last_autosave_timestamp: get_millis(),
			task_tick: None,
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

	pub fn launch_tick_task(chunk_system: &mut ChunkSystem, chunk_system_weak: ChunkSystemWeak) {
		chunk_system.task_tick = Some(
			tokio::task::Builder::new()
				.name("Chunk system task")
				.spawn(async move { ChunkSystem::tick_task_runner(chunk_system_weak).await })
				.unwrap(),
		);
	}

	async fn tick_task_runner(chunk_system_weak: ChunkSystemWeak) {
		while let Some(chunk_system) = chunk_system_weak.upgrade() {
			let mut chunk_system = chunk_system.lock().await;
			chunk_system.tick(&chunk_system_weak).await;
			drop(chunk_system);
			tokio::time::sleep(std::time::Duration::from_secs(1)).await;
		}
	}

	// Called every 1s
	async fn tick(&mut self, weak: &ChunkSystemWeak) {
		let time_ms = get_millis();
		if self.last_autosave_timestamp + (self.autosave_interval_ms as u64) < time_ms {
			self.last_autosave_timestamp = time_ms;

			let mut to_autosave: Vec<ChunkInstanceWeak> = Vec::new();
			for chunk in self.chunks.values() {
				if chunk.lock().await.is_modified() {
					to_autosave.push(Arc::downgrade(chunk));
				}
			}
			if !to_autosave.is_empty() {
				log::info!("Performing auto-save");
				ChunkSystem::save_chunks(to_autosave, weak.clone()).await
			} else {
				log::info!("Nothing to save")
			}
		}
	}

	async fn save_chunks(to_autosave: Vec<ChunkInstanceWeak>, weak: ChunkSystemWeak) {}

	pub async fn cleanup(&mut self) {
		if let Some(task) = &self.task_tick {
			task.abort();
		}
		self.cleaned_up = true;
	}
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;
pub type ChunkSystemWeak = Weak<Mutex<ChunkSystem>>;

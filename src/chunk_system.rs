use std::{
	collections::HashMap,
	sync::{Arc, Weak},
};

use glam::{IVec2, UVec2};
use tokio::{
	sync::{Mutex, Notify},
	task::JoinHandle,
};

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex, ChunkInstanceWeak},
	database::{Database, DatabaseFunc},
	limits::CHUNK_SIZE_PX,
	preview_system::PreviewSystemMutex,
	signal::Signal,
	time::get_millis,
};

pub struct ChunkSystem {
	chunks: HashMap<IVec2, ChunkInstanceMutex>,
	database: Arc<Mutex<Database>>,
	cleaned_up: bool,
	autosave_interval_ms: u32,
	last_autosave_timestamp: u64,
	task_processor: Option<JoinHandle<()>>,
	task_tick: Option<JoinHandle<()>>,
	preview_system: PreviewSystemMutex,
	notifier: Arc<Notify>,
	signal_garbage_collect: Signal,
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
	pub fn new(
		database: Arc<Mutex<Database>>,
		preview_system: PreviewSystemMutex,
		autosave_interval_ms: u32,
	) -> Self {
		let notifier = Arc::new(Notify::new());

		Self {
			chunks: HashMap::new(),
			database,
			cleaned_up: false,
			autosave_interval_ms,
			last_autosave_timestamp: get_millis(),
			preview_system,
			task_tick: None,
			task_processor: None,
			notifier: notifier.clone(),
			signal_garbage_collect: Signal::new(notifier),
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
		if let Some(data) = DatabaseFunc::chunk_load_data(&self.database, chunk_pos).await? {
			compressed_chunk_data = Some(data.data);
		}

		let queue_cache = self.preview_system.lock().await.update_queue_cache.clone();

		// Allocate chunk
		let chunk_mtx = Arc::new(Mutex::new(ChunkInstance::new(
			chunk_pos,
			queue_cache,
			self.signal_garbage_collect.clone(),
			compressed_chunk_data,
		)));

		self.chunks.insert(chunk_pos, chunk_mtx.clone());

		Ok(chunk_mtx)
	}

	pub fn launch_task_processor(chunk_system: &mut ChunkSystem, chunk_system_weak: ChunkSystemWeak) {
		chunk_system.task_processor = Some(
			tokio::task::Builder::new()
				.name("Chunk system processor task")
				.spawn(async move {
					while let Some(chunk_system_mtx) = chunk_system_weak.upgrade() {
						let mut chunk_system = chunk_system_mtx.lock().await;
						let notifier = chunk_system.notifier.clone();
						drop(chunk_system);

						notifier.notified().await;

						chunk_system = chunk_system_mtx.lock().await;

						if chunk_system.signal_garbage_collect.check_triggered() {
							chunk_system.garbage_collect().await;
						}
					}
					log::trace!("Chunk system processor task ended");
				})
				.unwrap(),
		);
	}

	pub async fn garbage_collect(&mut self) {
		let mut chunks_to_save: Vec<IVec2> = Vec::new();
		let mut chunks_to_free: Vec<IVec2> = Vec::new();

		for (chunk_pos, chunk_mtx) in &self.chunks {
			let chunk = chunk_mtx.lock().await;
			if chunk.is_linked_sessions_empty() {
				if chunk.is_modified() {
					chunks_to_save.push(*chunk_pos);
				}
				debug_assert!(Arc::strong_count(chunk_mtx) == 1);
				chunks_to_free.push(*chunk_pos);
			}
		}

		// Save pending chunks
		for chunk_pos in &chunks_to_save {
			if let Some(chunk) = self.chunks.get(chunk_pos) {
				let chunk = chunk.clone();
				let mut chunk = chunk.lock().await;
				self.save_chunk_wrapper(&mut chunk).await;
			}
		}

		// Free chunks
		for chunk_pos in &chunks_to_free {
			self.chunks.remove(chunk_pos);
		}

		if !chunks_to_save.is_empty() || !chunks_to_free.is_empty() {
			log::info!(
				"Garbage-collected chunks ({} saved, {} total loaded, {} freed)",
				chunks_to_save.len(),
				self.chunks.len(),
				chunks_to_free.len()
			);
		}
	}

	pub fn launch_task_tick(chunk_system: &mut ChunkSystem, chunk_system_weak: ChunkSystemWeak) {
		chunk_system.task_tick = Some(
			tokio::task::Builder::new()
				.name("Chunk system tick task")
				.spawn(async move {
					while chunk_system_weak.strong_count() > 0 {
						ChunkSystem::tick(&chunk_system_weak).await;
						tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					}
				})
				.unwrap(),
		);
	}

	async fn get_chunks_to_save(&self) -> Vec<ChunkInstanceWeak> {
		let mut to_autosave: Vec<ChunkInstanceWeak> = Vec::new();
		for chunk in self.chunks.values() {
			if chunk.lock().await.is_modified() {
				to_autosave.push(Arc::downgrade(chunk));
			}
		}
		to_autosave
	}

	// Called every 1s
	async fn tick(weak: &ChunkSystemWeak) {
		if let Some(chunk_sytem) = weak.upgrade() {
			let mut chunk_system = chunk_sytem.lock().await;

			let time_ms = get_millis();
			if chunk_system.last_autosave_timestamp + (chunk_system.autosave_interval_ms as u64) < time_ms
			{
				chunk_system.last_autosave_timestamp = time_ms;

				let to_autosave = chunk_system.get_chunks_to_save().await;
				if !to_autosave.is_empty() {
					log::info!("Performing auto-save");
					drop(chunk_system);
					ChunkSystem::save_chunks_lazy(to_autosave, weak.clone()).await;
				}
			}
		}
	}

	async fn save_chunk_wrapper(&mut self, chunk: &mut ChunkInstance) {
		log::info!("Saving chunk at {}x{}", chunk.position.x, chunk.position.y);
		if let Err(e) = self.save_chunk(chunk).await {
			log::error!(
				"Failed to save chunk at {}x{}: {}",
				chunk.position.x,
				chunk.position.y,
				e
			);
		}
	}

	async fn save_chunk(&mut self, chunk: &mut ChunkInstance) -> anyhow::Result<()> {
		let data = chunk.encode_chunk_data(true);

		DatabaseFunc::chunk_save_data(
			&self.database,
			chunk.position,
			data,
			crate::database::CompressionType::Lz4,
		)
		.await?;

		Ok(())
	}

	async fn save_chunks(&mut self, mut to_autosave: Vec<ChunkInstanceWeak>) {
		while let Some(chunk) = to_autosave.pop() {
			if let Some(chunk) = chunk.upgrade() {
				let mut chunk = chunk.lock().await;
				if !chunk.is_modified() {
					continue;
				}

				self.save_chunk_wrapper(&mut chunk).await;
			}
		}
	}

	async fn save_chunks_lazy(mut to_autosave: Vec<ChunkInstanceWeak>, weak: ChunkSystemWeak) {
		while let Some(chunk) = to_autosave.pop() {
			if let Some(chunk) = chunk.upgrade() {
				let mut chunk = chunk.lock().await;
				if !chunk.is_modified() {
					continue;
				}

				if let Some(chunk_system) = weak.upgrade() {
					let mut chunk_system = chunk_system.lock().await;
					chunk_system.save_chunk_wrapper(&mut chunk).await;
				}
			}
		}
	}

	pub async fn cleanup(&mut self) {
		if let Some(task) = &self.task_tick {
			task.abort();
		}

		// Save all modified chunks
		let to_autosave = self.get_chunks_to_save().await;
		if !to_autosave.is_empty() {
			log::info!("Saving chunks before exit");
			self.save_chunks(to_autosave).await;
			log::info!("Chunks saved!");
		}

		self.cleaned_up = true;
	}
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;
pub type ChunkSystemWeak = Weak<Mutex<ChunkSystem>>;

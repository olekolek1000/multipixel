use std::{
	collections::HashMap,
	sync::{Arc, Weak},
};

use std::sync::Mutex as SyncMutex;

use glam::{IVec2, UVec2};
use tokio::{
	sync::{Mutex, Notify},
	task::JoinHandle,
};

use crate::{
	chunk::{ChunkInstance, ChunkInstanceMutex, ChunkInstanceRefs, ChunkInstanceWeak},
	database::{Database, DatabaseFunc},
	limits::CHUNK_SIZE_PX,
	preview_system::{PreviewSystem, PreviewSystemMutex},
	signal::Signal,
	time::get_millis,
};

#[derive(Clone)]
struct ChunkCell {
	chunk: ChunkInstanceMutex,
	refs: ChunkInstanceRefs,
}

struct ChunkCellWeak {
	chunk: ChunkInstanceWeak,
	refs: ChunkInstanceRefs,
}

pub struct ChunkSystem {
	chunks: HashMap<IVec2, ChunkCell>,
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

struct GarbageCollectData {
	chunks_to_save: Vec<ChunkInstanceWeak>,
	chunks_to_free: Vec<IVec2>,
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
		if let Some(cell) = self.chunks.get(&chunk_pos) {
			// Return previously loaded chunk
			return Ok(cell.chunk.clone());
		}

		let mut compressed_chunk_data: Option<Vec<u8>> = None;

		// Load chunk pixels from the database
		if let Some(data) = DatabaseFunc::chunk_load_data(&self.database, chunk_pos).await? {
			compressed_chunk_data = Some(data.data);
		}

		let queue_cache = self.preview_system.lock().await.update_queue_cache.clone();

		let refs = ChunkInstanceRefs {
			modified: Arc::new(SyncMutex::new(false)),
			linked_sessions: Arc::new(SyncMutex::new(Vec::new())),
		};

		// Allocate chunk
		let chunk_mtx = Arc::new(Mutex::new(ChunkInstance::new(
			chunk_pos,
			refs.clone(),
			queue_cache,
			self.signal_garbage_collect.clone(),
			compressed_chunk_data,
		)));

		self.chunks.insert(
			chunk_pos,
			ChunkCell {
				chunk: chunk_mtx.clone(),
				refs,
			},
		);

		Ok(chunk_mtx)
	}

	pub fn launch_task_processor(chunk_system: &mut ChunkSystem, chunk_system_weak: ChunkSystemWeak) {
		chunk_system.task_processor = Some(
			tokio::task::Builder::new()
				.name("Chunk system processor task")
				.spawn(async move {
					while let Some(chunk_system_mtx) = chunk_system_weak.upgrade() {
						let chunk_system = chunk_system_mtx.lock().await;
						let database = chunk_system.database.clone();
						let notifier = chunk_system.notifier.clone();
						drop(chunk_system);

						notifier.notified().await;

						if let Some(data) =
							ChunkSystem::get_chunks_to_garbage_collect(chunk_system_mtx.clone()).await
						{
							ChunkSystem::garbage_collect_lazy(chunk_system_mtx, database, data).await;
						}
					}
					log::trace!("Chunk system processor task ended");
				})
				.unwrap(),
		);
	}

	async fn get_chunks_to_garbage_collect(
		chunk_system_mtx: ChunkSystemMutex,
	) -> Option<GarbageCollectData> {
		let chunk_system = chunk_system_mtx.lock().await;
		if !chunk_system.signal_garbage_collect.check_triggered() {
			return None;
		}

		let chunks: Vec<(IVec2, ChunkCellWeak)> = chunk_system
			.chunks
			.iter()
			.map(|cell| {
				(
					*cell.0,
					ChunkCellWeak {
						chunk: Arc::downgrade(&cell.1.chunk),
						refs: cell.1.refs.clone(),
					},
				)
			})
			.collect();

		drop(chunk_system);

		let mut chunks_to_save: Vec<ChunkInstanceWeak> = Vec::new();
		let mut chunks_to_free: Vec<IVec2> = Vec::new();
		for (chunk_pos, cell) in &chunks {
			if cell.refs.linked_sessions.lock().unwrap().is_empty() {
				if *cell.refs.modified.lock().unwrap() {
					chunks_to_save.push(cell.chunk.clone());
				}
				chunks_to_free.push(*chunk_pos);
			}
		}

		Some(GarbageCollectData {
			chunks_to_free,
			chunks_to_save,
		})
	}

	async fn garbage_collect_lazy(
		chunk_system_mtx: ChunkSystemMutex,
		database: Arc<Mutex<Database>>,
		data: GarbageCollectData,
	) {
		// Save pending chunks
		for chunk in &data.chunks_to_save {
			if let Some(chunk) = chunk.upgrade() {
				let chunk = chunk.clone();
				let mut chunk = chunk.lock().await;
				ChunkSystem::save_chunk_wrapper(database.clone(), &mut chunk).await;
			}
		}

		let total_loaded = {
			let mut chunk_system = chunk_system_mtx.lock().await;

			// Free chunks
			for chunk_pos in &data.chunks_to_free {
				chunk_system.chunks.remove(chunk_pos);
			}

			chunk_system.chunks.len()
		};

		if !data.chunks_to_save.is_empty() {
			log::trace!(
				"Garbage-collected chunks ({} saved, {} total loaded, {} freed)",
				data.chunks_to_save.len(),
				total_loaded,
				data.chunks_to_free.len()
			);
		}
	}

	pub fn launch_task_tick(chunk_system: &mut ChunkSystem, chunk_system_weak: ChunkSystemWeak) {
		chunk_system.task_tick = Some(
			tokio::task::Builder::new()
				.name("Chunk system tick task")
				.spawn(async move {
					while let Some(chunk_system_mtx) = chunk_system_weak.upgrade() {
						ChunkSystem::tick(chunk_system_mtx).await;
						tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					}
				})
				.unwrap(),
		);
	}

	async fn get_chunks_to_save(&self) -> Vec<ChunkCellWeak> {
		let mut to_autosave: Vec<ChunkCellWeak> = Vec::new();
		for cell in self.chunks.values() {
			// If modified
			if *cell.refs.modified.lock().unwrap() {
				to_autosave.push(ChunkCellWeak {
					chunk: Arc::downgrade(&cell.chunk),
					refs: cell.refs.clone(),
				});
			}
		}
		to_autosave
	}

	// Called every 1s
	async fn tick(chunk_system_mtx: ChunkSystemMutex) {
		let mut chunk_system = chunk_system_mtx.lock().await;
		let preview_system = chunk_system.preview_system.clone();

		let time_ms = get_millis();
		if chunk_system.last_autosave_timestamp + (chunk_system.autosave_interval_ms as u64) < time_ms {
			chunk_system.last_autosave_timestamp = time_ms;

			let to_autosave = chunk_system.get_chunks_to_save().await;
			if !to_autosave.is_empty() {
				log::info!("Performing auto-save");
				drop(chunk_system);
				ChunkSystem::save_chunks(chunk_system_mtx, to_autosave).await;
				log::info!("Auto-save finished");

				PreviewSystem::process_all(preview_system).await;
			}
		}
	}

	pub async fn regenerate_all_previews(chunk_system_mtx: ChunkSystemMutex) {
		let chunk_system = chunk_system_mtx.lock().await;
		let to_process: Vec<IVec2> = chunk_system.chunks.iter().map(|c| *c.0).collect();

		let preview_system_mtx = chunk_system.preview_system.clone();
		drop(chunk_system);

		let queue_cache = preview_system_mtx.lock().await.update_queue_cache.clone();
		for cell in to_process {
			queue_cache.send(cell);
		}

		PreviewSystem::process_all(preview_system_mtx).await;
	}

	async fn save_chunk_wrapper(database: Arc<Mutex<Database>>, chunk: &mut ChunkInstance) {
		log::trace!("Saving chunk at {}x{}", chunk.position.x, chunk.position.y);
		if let Err(e) = ChunkSystem::save_chunk(database, chunk).await {
			log::error!(
				"Failed to save chunk at {}x{}: {}",
				chunk.position.x,
				chunk.position.y,
				e
			);
		}
	}

	async fn save_chunk(
		database: Arc<Mutex<Database>>,
		chunk: &mut ChunkInstance,
	) -> anyhow::Result<()> {
		let data = chunk.encode_chunk_data(true);

		DatabaseFunc::chunk_save_data(
			&database,
			chunk.position,
			data,
			crate::database::CompressionType::Lz4,
		)
		.await?;

		Ok(())
	}

	async fn save_chunks(chunk_system_mtx: ChunkSystemMutex, mut to_autosave: Vec<ChunkCellWeak>) {
		while let Some(cell) = to_autosave.pop() {
			if let Some(chunk) = cell.chunk.upgrade() {
				let mut chunk = chunk.lock().await;
				if !*cell.refs.modified.lock().unwrap() {
					continue;
				}

				let chunk_system = chunk_system_mtx.lock().await;
				let database = chunk_system.database.clone();
				drop(chunk_system);

				ChunkSystem::save_chunk_wrapper(database.clone(), &mut chunk).await;
			}
		}
	}

	pub async fn cleanup(chunk_system_mtx: ChunkSystemMutex) {
		let chunk_system = chunk_system_mtx.lock().await;
		if let Some(task) = &chunk_system.task_tick {
			task.abort();
		}

		// Save all modified chunks
		let to_autosave = chunk_system.get_chunks_to_save().await;
		drop(chunk_system);
		if !to_autosave.is_empty() {
			log::info!("Saving chunks before exit");
			ChunkSystem::save_chunks(chunk_system_mtx.clone(), to_autosave).await;
			log::info!("Chunks saved!");
		}

		chunk_system_mtx.lock().await.cleaned_up = true;
	}
}

pub type ChunkSystemMutex = Arc<Mutex<ChunkSystem>>;
pub type ChunkSystemWeak = Weak<Mutex<ChunkSystem>>;

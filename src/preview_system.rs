#![allow(dead_code)]

use std::sync::{Arc, Weak};

use anyhow::anyhow;
use glam::IVec2;
use tokio::sync::{Mutex, Notify};

use crate::{
	compression,
	database::{ChunkDatabaseRecord, Database, DatabaseFunc, PreviewDatabaseRecord},
	event_queue::EventQueue,
	limits::{self, CHUNK_SIZE_PX},
};

#[derive(Default, Clone)]
pub struct PreviewSystemLayer {
	zoom: u8,
	update_queue: Vec<IVec2>,
}

fn extract_chunk_record(record: Option<ChunkDatabaseRecord>) -> Vec<u8> {
	if let Some(record) = record {
		record.data
	} else {
		Vec::new()
	}
}

fn extract_preview_record(record: Option<PreviewDatabaseRecord>) -> Vec<u8> {
	if let Some(record) = record {
		record.data
	} else {
		Vec::new()
	}
}

fn decompress_vec_lz4(compressed: &[u8]) -> anyhow::Result<Vec<u8>> {
	if compressed.is_empty() {
		return Ok(Vec::new());
	}
	if let Some(decompressed) =
		compression::decompress_lz4(compressed, (CHUNK_SIZE_PX * CHUNK_SIZE_PX * 3) as usize)
	{
		Ok(decompressed)
	} else {
		Err(anyhow!("Failed to decompress LZ4 data"))
	}
}

fn fill_image(out_data: &mut [u8], in_data: &[u8], x: u32, y: u32) {
	if in_data.is_empty() {
		return; // blank
	}

	let offset_x = CHUNK_SIZE_PX * x;
	let offset_y = CHUNK_SIZE_PX * y;

	let image_size = CHUNK_SIZE_PX * 2;
	let pitch_in = CHUNK_SIZE_PX * 3;
	let pitch_out = image_size * 3;

	for local_y in 0..CHUNK_SIZE_PX {
		for local_x in 0..CHUNK_SIZE_PX {
			let target_x = offset_x + local_x;
			let target_y = offset_y + local_y;

			let offset_in = local_y * pitch_in + local_x * 3;
			let offset_out = target_y * pitch_out + target_x * 3;

			out_data[(offset_out) as usize] = in_data[(offset_in) as usize];
			out_data[(offset_out + 1) as usize] = in_data[(offset_in + 1) as usize];
			out_data[(offset_out + 2) as usize] = in_data[(offset_in + 2) as usize];
		}
	}
}

struct PreviewProcessResult {
	has_more: bool,
	upper_pos: IVec2,
}

impl PreviewSystemLayer {
	pub fn new(zoom: u8) -> Self {
		Self {
			zoom,
			update_queue: Vec::new(),
		}
	}

	pub fn add_to_queue(&mut self, coords: &IVec2) {
		// Check if already added to queue (reverse iterator)
		for it in self.update_queue.iter().rev() {
			if *it == *coords {
				return; // Already added to the queue
			}
		}

		self.update_queue.push(*coords);
	}

	async fn process_one_block(
		database: &Arc<Mutex<Database>>,
		update_queue: &mut Vec<IVec2>,
		zoom: u8,
	) -> anyhow::Result<PreviewProcessResult> {
		if let Some(position) = update_queue.pop() {
			// Fuse 2x2 chunks into one preview image
			let topleft = IVec2::new(position.x * 2, position.y * 2);
			let topright = IVec2::new(position.x * 2 + 1, position.y * 2);
			let bottomleft = IVec2::new(position.x * 2, position.y * 2 + 1);
			let bottomright = IVec2::new(position.x * 2 + 1, position.y * 2 + 1);

			struct Quad {
				topleft: Vec<u8>,
				topright: Vec<u8>,
				bottomleft: Vec<u8>,
				bottomright: Vec<u8>,
			}

			let compressed = if zoom == 1 {
				// Load real chunks data underneath
				Quad {
					topleft: extract_chunk_record(DatabaseFunc::chunk_load_data(database, topleft).await?),
					topright: extract_chunk_record(DatabaseFunc::chunk_load_data(database, topright).await?),
					bottomleft: extract_chunk_record(
						DatabaseFunc::chunk_load_data(database, bottomleft).await?,
					),
					bottomright: extract_chunk_record(
						DatabaseFunc::chunk_load_data(database, bottomright).await?,
					),
				}
			} else {
				// Load preview system layer chunks
				Quad {
					topleft: extract_preview_record(
						DatabaseFunc::preview_load_data(database, topleft, zoom - 1).await?,
					),
					topright: extract_preview_record(
						DatabaseFunc::preview_load_data(database, topright, zoom - 1).await?,
					),
					bottomleft: extract_preview_record(
						DatabaseFunc::preview_load_data(database, bottomleft, zoom - 1).await?,
					),
					bottomright: extract_preview_record(
						DatabaseFunc::preview_load_data(database, bottomright, zoom - 1).await?,
					),
				}
			};

			// Decompress data
			let data_topleft = decompress_vec_lz4(&compressed.topleft)?;
			let data_topright = decompress_vec_lz4(&compressed.topright)?;
			let data_bottomleft = decompress_vec_lz4(&compressed.bottomleft)?;
			let data_bottomright = decompress_vec_lz4(&compressed.bottomright)?;

			// Allocate preview chunk data
			let image_size = CHUNK_SIZE_PX * 2;
			let mut rgb: Vec<u8> = Vec::new();
			rgb.resize(
				(image_size * image_size /* 512² */ * 3/*RGB*/) as usize,
				255, /* Fill with white */
			);

			// Blit images
			fill_image(&mut rgb, &data_topleft, 0, 0);
			fill_image(&mut rgb, &data_topright, 1, 0);
			fill_image(&mut rgb, &data_bottomleft, 0, 1);
			fill_image(&mut rgb, &data_bottomright, 1, 1);

			// Downscale image
			let mut downscaled: Vec<u8> = vec![0; (CHUNK_SIZE_PX * CHUNK_SIZE_PX * 3) as usize];

			let downscaled_pitch = CHUNK_SIZE_PX * 3;
			let image_pitch = image_size * 3;
			for y in 0..CHUNK_SIZE_PX {
				for x in 0..CHUNK_SIZE_PX {
					let in_x = x * 2;
					let in_y = y * 2;

					let mut perform_channel = |channel: u32| {
						// Same four pixels
						let result = (rgb[((in_y) * image_pitch + (in_x) * 3 + channel) as usize] as u32
							+ rgb[((in_y + 1) * image_pitch + (in_x) * 3 + channel) as usize] as u32
							+ rgb[((in_y) * image_pitch + (in_x + 1) * 3 + channel) as usize] as u32
							+ rgb[((in_y + 1) * image_pitch + (in_x + 1) * 3 + channel) as usize] as u32)
							/ 4;

						// Save sampled pixel result for specific channel
						downscaled[(y * downscaled_pitch + x * 3 + channel) as usize] = result as u8;
					};

					perform_channel(0); // Red
					perform_channel(1); // Green
					perform_channel(2); // Blue
				}
			}

			// Compress downscaled image
			let compressed = compression::compress_lz4(&downscaled);

			DatabaseFunc::preview_save_data(database, position, zoom, compressed).await?;

			let upper_pos = IVec2::new(
				/* X */
				if position.x >= 0 {
					position.x / 2
				} else {
					(position.x - 1) / 2
				},
				/* Y */
				if position.y >= 0 {
					position.y / 2
				} else {
					(position.y - 1) / 2
				},
			);

			log::trace!(
				"Processed block at {}x{}, zoom {} ({} remaining)",
				position.x,
				position.y,
				zoom,
				update_queue.len()
			);

			Ok(PreviewProcessResult {
				has_more: true,
				upper_pos,
			})
		} else {
			// Nothing to do
			Ok(PreviewProcessResult {
				has_more: false,
				upper_pos: IVec2::ZERO,
			})
		}
	}
}

pub type PreviewSystemQueuedChunks = EventQueue<IVec2>;

pub struct PreviewSystem {
	//layers: Vec<PreviewSystemLayer>,
	database: Arc<Mutex<Database>>,

	layers: Vec<PreviewSystemLayer>,

	notifier: Arc<Notify>,
	pub update_queue_cache: PreviewSystemQueuedChunks,
}

fn layer_index_to_zoom(index: u8) -> u8 {
	index + 1
}

fn init_layers_vec() -> Vec<PreviewSystemLayer> {
	let mut layers: Vec<PreviewSystemLayer> = Vec::new();
	for i in 0..limits::PREVIEW_SYSTEM_LAYER_COUNT {
		layers.push(PreviewSystemLayer::new(layer_index_to_zoom(i)));
	}
	layers
}

impl PreviewSystem {
	pub fn new(database: Arc<Mutex<Database>>) -> Self {
		let notifier = Arc::new(Notify::new());

		// Init layers
		let mut layers: Vec<PreviewSystemLayer> = Vec::new();
		for i in 0..limits::PREVIEW_SYSTEM_LAYER_COUNT {
			layers.push(PreviewSystemLayer::new(layer_index_to_zoom(i)));
		}

		Self {
			database,
			notifier: notifier.clone(),
			update_queue_cache: PreviewSystemQueuedChunks::new(notifier),
			layers: init_layers_vec(),
		}
	}

	fn layer_index_to_zoom(index: u8) -> u8 {
		index + 1
	}

	pub async fn request_data(&self, pos: &IVec2, zoom: u8) -> anyhow::Result<Option<Arc<Vec<u8>>>> {
		if let Some(record) = DatabaseFunc::preview_load_data(&self.database, *pos, zoom).await? {
			Ok(Some(Arc::new(record.data)))
		} else {
			Ok(None)
		}
	}

	async fn process_queue(&mut self) {
		for pos in &self.update_queue_cache.read_all() {
			self.layers[0].add_to_queue(pos);
		}
	}

	async fn process_layers(preview_system_weak: PreviewSystemWeak) -> anyhow::Result<()> {
		let preview_system_mtx = preview_system_weak
			.upgrade()
			.ok_or(anyhow!("Preview system expired"))?;
		let mut preview_system = preview_system_mtx.lock().await;
		let mut layers = std::mem::take(&mut preview_system.layers);
		preview_system.layers = init_layers_vec();
		let db_weak = Arc::downgrade(&preview_system.database);
		drop(preview_system);
		drop(preview_system_mtx);

		for i in 0..layers.len() {
			loop {
				if let Some(db) = db_weak.upgrade() {
					let layer = &mut layers[i];
					let res =
						PreviewSystemLayer::process_one_block(&db, &mut layer.update_queue, layer.zoom).await?;

					if !res.has_more {
						break;
					}

					if i < layers.len() - 1 {
						let upper_layer = &mut layers[i + 1];
						upper_layer.add_to_queue(&res.upper_pos);
					}
				} else {
					return Err(anyhow!("Database expired"));
				}
			}
		}

		Ok(())
	}

	pub async fn process_all(preview_system_mtx: PreviewSystemMutex) {
		log::trace!("Processing all pending previews");
		let mut preview_system = preview_system_mtx.lock().await;
		preview_system.process_queue().await;
		drop(preview_system);
		let weak = Arc::downgrade(&preview_system_mtx);
		drop(preview_system_mtx);

		if let Err(e) = PreviewSystem::process_layers(weak).await {
			// This shouldn't happen on non-corrupted database anyways
			log::error!("Failed to process previews: {e}");
		}
	}
}

impl Drop for PreviewSystem {
	fn drop(&mut self) {
		log::trace!("Preview system freed")
	}
}

pub type PreviewSystemMutex = Arc<Mutex<PreviewSystem>>;
pub type PreviewSystemWeak = Weak<Mutex<PreviewSystem>>;

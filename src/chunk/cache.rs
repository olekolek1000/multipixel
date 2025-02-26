use std::sync::Arc;

use glam::{IVec2, U8Vec2};

use crate::pixel::GlobalPixelRGBA;

use super::{
	chunk::{ChunkInstanceMutex, ChunkInstanceWeak, ChunkPixelRGBA},
	compositor::LayerID,
	system::ChunkSystemMutex,
	writer::ChunkWriterRGBA,
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
			drop(chunk_system);
			self.chunk = Arc::downgrade(&chunk);
			self.chunk_pos = Some(chunk_pos);
			return Some(chunk);
		}

		None
	}

	pub async fn set_pixels_for_layer(
		&mut self,
		chunk_system_mtx: &ChunkSystemMutex,
		layer_id: LayerID,
		pixels: &[GlobalPixelRGBA],
	) {
		let mut writer = ChunkWriterRGBA::new();

		writer
			.generate_affected(pixels, self, chunk_system_mtx)
			.await;

		// For every affected chunk
		for cell in &writer.affected_chunks {
			if cell.queued_pixels.is_empty() {
				continue;
			}

			let mut chunk = cell.chunk.lock().await;

			let layer = chunk.compositor.get_or_alloc_mut(&layer_id);
			if layer.layer.read().is_none() {
				layer.layer.alloc_blank();
			}

			let queued_pixels: Vec<ChunkPixelRGBA> = cell
				.queued_pixels
				.iter()
				.map(|c| ChunkPixelRGBA {
					pos: c.0.pos,
					color: c.0.color,
				})
				.collect();

			layer.layer.set_pixels(&queued_pixels);

			let pixel_updates: Vec<U8Vec2> = cell.queued_pixels.iter().map(|c| c.0.pos).collect();
			chunk.allocate_image();
			chunk.send_pixel_updates(&pixel_updates);
		}
	}
}

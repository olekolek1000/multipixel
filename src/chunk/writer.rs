use glam::IVec2;

use crate::pixel::{GlobalPixelRGB, GlobalPixelRGBA};

use super::{
	cache::ChunkCache,
	chunk::{ChunkInstanceMutex, ChunkPixelRGB, ChunkPixelRGBA},
	system::{ChunkSystem, ChunkSystemMutex},
};

// T: GlobalPixelRGB or GlobalPixelRGBA
pub struct ChunkCacheCell<T> {
	pub chunk_pos: IVec2,
	pub chunk: ChunkInstanceMutex,
	pub queued_pixels: Vec<(
		T,     /* local pixel position */
		IVec2, /* global pixel position */
	)>,
}

pub struct ChunkWriter<T> {
	pub affected_chunks: Vec<ChunkCacheCell<T>>,
}

pub type ChunkWriterRGB = ChunkWriter<ChunkPixelRGB>;
pub type ChunkWriterRGBA = ChunkWriter<ChunkPixelRGBA>;

impl<T> ChunkWriter<T> {
	pub fn new() -> Self {
		Self {
			affected_chunks: Vec::new(),
		}
	}

	fn fetch_cell<'a>(
		affected_chunks: &'a mut [ChunkCacheCell<T>],
		chunk_pos: &IVec2,
	) -> Option<&'a mut ChunkCacheCell<T>> {
		affected_chunks
			.iter_mut()
			.find(|cell| cell.chunk_pos == *chunk_pos)
	}

	async fn cache_new_chunk(
		chunk_cache: &mut ChunkCache,
		chunk_system_mtx: &ChunkSystemMutex,
		affected_chunks: &mut Vec<ChunkCacheCell<T>>,
		chunk_pos: &IVec2,
	) {
		if let Some(chunk) = chunk_cache.get(chunk_system_mtx, *chunk_pos).await {
			affected_chunks.push(ChunkCacheCell::<T> {
				chunk_pos: *chunk_pos,
				chunk,
				queued_pixels: Vec::new(),
			});
		}
	}
}

impl ChunkWriterRGB {
	pub async fn generate_affected(
		&mut self,
		pixels: &[GlobalPixelRGB],
		chunk_cache: &mut ChunkCache,
		chunk_system_mtx: &ChunkSystemMutex,
	) {
		// Generate affected chunks list
		for pixel in pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if Self::fetch_cell(&mut self.affected_chunks, &chunk_pos).is_none() {
				Self::cache_new_chunk(
					chunk_cache,
					chunk_system_mtx,
					&mut self.affected_chunks,
					&chunk_pos,
				)
				.await;
			}
		}

		// Queue pixels to send
		for pixel in pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if let Some(cell) = ChunkWriterRGB::fetch_cell(&mut self.affected_chunks, &chunk_pos) {
				cell.queued_pixels.push((
					ChunkPixelRGB {
						color: pixel.color,
						pos: ChunkSystem::global_pixel_pos_to_local_pixel_pos(pixel.pos),
					},
					pixel.pos,
				));
			} else {
				// Skip pixel, already set
				continue;
			}
		}
	}
}

impl ChunkWriterRGBA {
	pub async fn generate_affected(
		&mut self,
		pixels: &[GlobalPixelRGBA],
		chunk_cache: &mut ChunkCache,
		chunk_system_mtx: &ChunkSystemMutex,
	) {
		// Generate affected chunks list
		for pixel in pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if Self::fetch_cell(&mut self.affected_chunks, &chunk_pos).is_none() {
				Self::cache_new_chunk(
					chunk_cache,
					chunk_system_mtx,
					&mut self.affected_chunks,
					&chunk_pos,
				)
				.await;
			}
		}

		// Queue pixels to send
		for pixel in pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if let Some(cell) = ChunkWriterRGBA::fetch_cell(&mut self.affected_chunks, &chunk_pos) {
				cell.queued_pixels.push((
					ChunkPixelRGBA {
						color: pixel.color,
						pos: ChunkSystem::global_pixel_pos_to_local_pixel_pos(pixel.pos),
					},
					pixel.pos,
				));
			} else {
				// Skip pixel, already set
				continue;
			}
		}
	}
}

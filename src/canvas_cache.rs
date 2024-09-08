use std::collections::HashMap;

use glam::IVec2;

use crate::{
	chunk_system::{ChunkSystem, ChunkSystemMutex},
	limits::CHUNK_SIZE_PX,
	pixel::Color,
};

struct Cell {
	pub raw_image_data: Vec<u8>, // same size as chunk data
}

#[derive(Default)]
pub struct CanvasCache {
	cells: HashMap<IVec2, Cell>,
}

impl CanvasCache {
	async fn get_cell_mut(
		&mut self,
		chunk_system_mtx: &ChunkSystemMutex,
		chunk_pos: &IVec2,
	) -> Option<&mut Cell> {
		if self.cells.contains_key(chunk_pos) {
			self.cells.get_mut(chunk_pos)
		} else {
			let mut chunk_system = chunk_system_mtx.lock().await;
			if let Ok(chunk) = chunk_system.get_chunk(*chunk_pos).await {
				drop(chunk_system);
				let mut chunk = chunk.lock().await;
				chunk.allocate_image();
				if let Some(data) = &chunk.raw_image_data {
					self.cells.insert(
						*chunk_pos,
						Cell {
							raw_image_data: data.clone(),
						},
					);
					return self.cells.get_mut(chunk_pos);
				}
			}
			// should never happen
			None
		}
	}

	pub async fn get_pixel(
		&mut self,
		chunk_system_mtx: &ChunkSystemMutex,
		global_pos: &IVec2,
	) -> Color {
		let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(*global_pos);
		if let Some(cell) = self.get_cell_mut(chunk_system_mtx, &chunk_pos).await {
			let local_pos = ChunkSystem::global_pixel_pos_to_local_pixel_pos(*global_pos);
			let offset = (local_pos.y * CHUNK_SIZE_PX * 3 + local_pos.x * 3) as usize;
			Color {
				r: cell.raw_image_data[offset],
				g: cell.raw_image_data[offset + 1],
				b: cell.raw_image_data[offset + 2],
			}
		} else {
			Color {
				..Default::default()
			}
		}
	}

	pub fn set_pixel(&mut self, global_pos: &IVec2, color: &Color) {
		let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(*global_pos);
		if let Some(cell) = self.cells.get_mut(&chunk_pos) {
			let local_pos = ChunkSystem::global_pixel_pos_to_local_pixel_pos(*global_pos);
			let offset = (local_pos.y * CHUNK_SIZE_PX * 3 + local_pos.x * 3) as usize;
			cell.raw_image_data[offset] = color.r;
			cell.raw_image_data[offset + 1] = color.g;
			cell.raw_image_data[offset + 2] = color.b;
		}
	}
}

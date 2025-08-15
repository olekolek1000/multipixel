use std::collections::HashMap;

use glam::IVec2;

use crate::{
	chunk::{
		layer::RGBAData,
		system::{ChunkSystem, ChunkSystemMutex},
	},
	limits::CHUNK_SIZE_PX,
	pixel::ColorRGBA,
};

struct Cell {
	pub data: RGBAData, // same size as chunk data
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
				let data = chunk.get_layer_main().read_unchecked();

				self.cells.insert(*chunk_pos, Cell { data: data.clone() });
				return self.cells.get_mut(chunk_pos);
			}
			// should never happen
			None
		}
	}

	pub async fn get_pixel(
		&mut self,
		chunk_system_mtx: &ChunkSystemMutex,
		global_pos: &IVec2,
	) -> ColorRGBA {
		let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(*global_pos);
		if let Some(cell) = self.get_cell_mut(chunk_system_mtx, &chunk_pos).await {
			let local_pos = ChunkSystem::global_pixel_pos_to_local_pixel_pos(*global_pos);
			let offset = (local_pos.y as u32 * CHUNK_SIZE_PX * 4 + local_pos.x as u32 * 4) as usize;
			ColorRGBA {
				r: cell.data.0[offset],
				g: cell.data.0[offset + 1],
				b: cell.data.0[offset + 2],
				a: cell.data.0[offset + 3],
			}
		} else {
			ColorRGBA {
				..Default::default()
			}
		}
	}

	pub fn set_pixel(&mut self, global_pos: &IVec2, color: &ColorRGBA) {
		let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(*global_pos);
		if let Some(cell) = self.cells.get_mut(&chunk_pos) {
			let local_pos = ChunkSystem::global_pixel_pos_to_local_pixel_pos(*global_pos);
			let offset = (local_pos.y as u32 * CHUNK_SIZE_PX * 4 + local_pos.x as u32 * 4) as usize;
			cell.data.0[offset] = color.r;
			cell.data.0[offset + 1] = color.g;
			cell.data.0[offset + 2] = color.b;
			cell.data.0[offset + 3] = color.a;
		}
	}
}

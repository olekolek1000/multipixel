use glam::U8Vec2;

use crate::{
	compression,
	limits::{self, CHUNK_SIZE_PX},
	pixel::ColorRGBA,
};

use super::chunk::ChunkPixelRGBA;

#[derive(Clone)]
pub struct RGBAData(pub Vec<u8>);

pub struct LayerRGBA {
	pub data: RGBAData,
}

impl LayerRGBA {
	pub const fn new() -> Self {
		Self {
			data: RGBAData(Vec::new()),
		}
	}

	pub const fn read(&self) -> Option<&RGBAData> {
		if self.data.0.is_empty() {
			None
		} else {
			Some(&self.data)
		}
	}

	pub fn set_data(&mut self, data: RGBAData) {
		self.data = data;
	}

	pub const fn read_unchecked(&self) -> &RGBAData {
		&self.data
	}

	pub fn compress_lz4(&self) -> Vec<u8> {
		compression::compress_lz4(&self.data.0)
	}

	pub const fn read_unchecked_mut(&mut self) -> &mut RGBAData {
		&mut self.data
	}

	pub fn alloc_transparent_black(&mut self) {
		let data: Vec<u8> = vec![0; limits::CHUNK_IMAGE_SIZE_BYTES_RGBA];
		self.data = RGBAData(data);
	}

	pub fn free(&mut self) {
		self.data = RGBAData(Vec::new());
	}

	/// Chunk needs to be allocated first!
	pub fn get_pixel(&self, chunk_pixel_pos: U8Vec2) -> ColorRGBA {
		debug_assert!(!self.data.0.is_empty());

		let data = self.read_unchecked();
		let offset = (u32::from(chunk_pixel_pos.y) * CHUNK_SIZE_PX * 4
			+ u32::from(chunk_pixel_pos.x) * 4) as usize;

		let rgba = data.0.as_ptr();

		unsafe {
			ColorRGBA {
				r: *rgba.add(offset),
				g: *rgba.add(offset + 1),
				b: *rgba.add(offset + 2),
				a: *rgba.add(offset + 3),
			}
		}
	}

	/// Chunk needs to be allocated first!
	pub fn set_pixel(&mut self, chunk_pixel_pos: U8Vec2, color: ColorRGBA) {
		debug_assert!(!self.data.0.is_empty());

		let data = self.read_unchecked_mut();
		let offset = (u32::from(chunk_pixel_pos.y) * (CHUNK_SIZE_PX * 4)
			+ u32::from(chunk_pixel_pos.x) * 4) as usize;

		let rgba = data.0.as_mut_ptr();

		unsafe {
			*rgba.add(offset) = color.r;
			*rgba.add(offset + 1) = color.g;
			*rgba.add(offset + 2) = color.b;
			*rgba.add(offset + 3) = color.a;
		}
	}

	#[allow(dead_code)]
	pub fn set_pixels(&mut self, pixels: &[ChunkPixelRGBA]) {
		let data = self.read_unchecked_mut();

		unsafe {
			let rgba = data.0.as_mut_ptr();

			for pixel in pixels {
				// Update pixel
				let offset =
					(u32::from(pixel.pos.y) * CHUNK_SIZE_PX * 4 + u32::from(pixel.pos.x) * 4) as usize;
				*rgba.add(offset) = pixel.color.r;
				*rgba.add(offset + 1) = pixel.color.g;
				*rgba.add(offset + 2) = pixel.color.b;
				*rgba.add(offset + 3) = pixel.color.a;
			}
		}
	}
}

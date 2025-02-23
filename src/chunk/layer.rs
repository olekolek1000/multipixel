use glam::UVec2;

use crate::{
	limits::{self, CHUNK_SIZE_PX},
	pixel::{ColorRGB, ColorRGBA},
};

use super::chunk::ChunkPixelRGBA;

#[derive(Clone)]
pub struct RGBData(pub Vec<u8>);

#[derive(Clone)]
pub struct RGBAData(pub Vec<u8>);

pub struct LayerRGB {
	pub data: RGBData,
}

pub struct LayerRGBA {
	data: RGBAData,
}

impl LayerRGB {
	pub fn new() -> Self {
		Self {
			data: RGBData(Vec::new()),
		}
	}

	pub fn read(&self) -> Option<&RGBData> {
		if self.data.0.is_empty() {
			None
		} else {
			Some(&self.data)
		}
	}

	pub fn read_unchecked(&self) -> &RGBData {
		&self.data
	}

	pub fn read_unchecked_mut(&mut self) -> &mut RGBData {
		&mut self.data
	}

	pub fn apply(&mut self, data: RGBData) {
		self.data = data;
	}

	pub fn alloc_white(&mut self) {
		let mut data = Vec::<u8>::new();
		data.resize(limits::CHUNK_IMAGE_SIZE_BYTES_RGB, 255); // White color
		self.data = RGBData(data);
	}

	pub fn free(&mut self) {
		self.data = RGBData(Vec::new());
	}

	/// Chunk needs to be allocated first!
	pub fn get_pixel(&self, chunk_pixel_pos: UVec2) -> ColorRGB {
		debug_assert!(!self.data.0.is_empty());

		let data = self.read_unchecked();
		let offset = (chunk_pixel_pos.y * CHUNK_SIZE_PX * 3 + chunk_pixel_pos.x * 3) as usize;

		ColorRGB {
			r: (data.0)[offset],
			g: (data.0)[offset + 1],
			b: (data.0)[offset + 2],
		}
	}
}

impl LayerRGBA {
	pub fn new() -> Self {
		Self {
			data: RGBAData(Vec::new()),
		}
	}

	pub fn read(&self) -> Option<&RGBAData> {
		if self.data.0.is_empty() {
			None
		} else {
			Some(&self.data)
		}
	}

	pub fn read_unchecked(&self) -> &RGBAData {
		&self.data
	}

	pub fn read_unchecked_mut(&mut self) -> &mut RGBAData {
		&mut self.data
	}

	pub fn alloc_blank(&mut self) {
		let data: Vec<u8> = vec![0; limits::CHUNK_IMAGE_SIZE_BYTES_RGBA];
		self.data = RGBAData(data);
	}

	/// Chunk needs to be allocated first!
	pub fn get_pixel(&self, chunk_pixel_pos: UVec2) -> ColorRGBA {
		debug_assert!(!self.data.0.is_empty());

		let data = self.read_unchecked();
		let offset = (chunk_pixel_pos.y * CHUNK_SIZE_PX * 4 + chunk_pixel_pos.x * 4) as usize;

		ColorRGBA {
			r: (data.0)[offset],
			g: (data.0)[offset + 1],
			b: (data.0)[offset + 2],
			a: (data.0)[offset + 3],
		}
	}

	/// Chunk needs to be allocated first!
	#[allow(dead_code)]
	pub fn set_pixel(&mut self, chunk_pixel_pos: UVec2, color: ColorRGBA) {
		debug_assert!(!self.data.0.is_empty());

		let data = self.read_unchecked_mut();
		let offset = (chunk_pixel_pos.y * (CHUNK_SIZE_PX * 4) + chunk_pixel_pos.x * 4) as usize;

		data.0[offset] = color.r;
		data.0[offset + 1] = color.g;
		data.0[offset + 2] = color.b;
		data.0[offset + 3] = color.a;
	}

	pub fn set_pixels(&mut self, pixels: &[ChunkPixelRGBA]) {
		let data = self.read_unchecked_mut();

		for pixel in pixels {
			// Update pixel
			let offset = (pixel.pos.y * CHUNK_SIZE_PX * 4 + pixel.pos.x * 4) as usize;
			(data.0)[offset] = pixel.color.r;
			(data.0)[offset + 1] = pixel.color.g;
			(data.0)[offset + 2] = pixel.color.b;
			(data.0)[offset + 3] = pixel.color.a;
		}
	}
}

use std::collections::HashSet;

use glam::IVec2;

use crate::{
	chunk::{cache::ChunkCache, compositor::LayerID, system::ChunkSystemSignal},
	pixel::{ColorRGB, ColorRGBA, GlobalPixelRGBA},
	room::RoomRefs,
};

use super::iter::LineMoveIter;

#[derive(Eq, PartialEq)]
pub struct ToolStateLine {
	start_pos: IVec2,
	target_prev: Option<IVec2>,
	layer_id: LayerID,
	affected_pixels: HashSet<IVec2>,
}

impl ToolStateLine {
	pub fn new(start_pos: IVec2, layer_id: LayerID) -> Self {
		Self {
			start_pos,
			layer_id,
			target_prev: None,
			affected_pixels: HashSet::new(),
		}
	}

	pub fn gen_global_pixel_vec_rgba(&self, color: ColorRGBA) -> Vec<GlobalPixelRGBA> {
		self
			.affected_pixels
			.iter()
			.map(|pos| GlobalPixelRGBA { pos: *pos, color })
			.collect()
	}

	pub fn cleanup(&self, refs: &RoomRefs) {
		refs
			.chunk_system_sender
			.send(ChunkSystemSignal::RemoveLayer(self.layer_id.clone()));
	}

	pub async fn process(
		&mut self,
		chunk_cache: &mut ChunkCache,
		refs: &RoomRefs,
		target: IVec2,
		color: ColorRGB,
	) {
		if let Some(target_prev) = self.target_prev {
			if (target - self.start_pos).abs().element_sum() > 2000 {
				// Too big distance!!
				self.target_prev = Some(target);
				return;
			}

			if target_prev == target {
				return; // nothing changed, do not re-render
			}
		}

		self.target_prev = Some(target);

		let iter = LineMoveIter::iterate(self.start_pos, target);

		// Clear previous iteration with transparent pixels
		let mut out_pixels = self.gen_global_pixel_vec_rgba(ColorRGBA::zero());
		self.affected_pixels.clear(); // generate line pixels from scratch

		for line in iter {
			self.affected_pixels.insert(line.pos);
		}

		out_pixels.extend(self.affected_pixels.iter().map(|c| GlobalPixelRGBA {
			color: color.rgba(255),
			pos: *c,
		}));

		chunk_cache
			.set_pixels_for_layer(&refs.chunk_system_mtx, self.layer_id.clone(), &out_pixels)
			.await;
	}
}

#[derive(Eq, PartialEq)]
pub enum ToolState {
	None,
	Line(ToolStateLine),
}

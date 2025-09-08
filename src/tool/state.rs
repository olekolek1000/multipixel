use std::{collections::HashSet, f32::consts::PI};

use glam::IVec2;

use crate::{
	chunk::{cache::ChunkCache, compositor::LayerID, system::ChunkSystemSignal},
	pixel::{ColorRGB, ColorRGBA, GlobalPixelRGBA},
	room::RoomRefs,
	tool::{
		brush::{BrushShape, ShapeType},
		triangle::TriangleRasterizerIter,
	},
	util,
};

use super::iter_line::LineMoveIter;

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

	fn write_line_iter(&mut self, iter: LineMoveIter) {
		for line in iter {
			self.affected_pixels.insert(line.pos);
		}
	}

	fn gen_pixels_1px(&mut self, target: IVec2) {
		self.write_line_iter(LineMoveIter::iterate(self.start_pos, target));
	}

	fn gen_pixels_thick(&mut self, target: IVec2, size: u8) {
		let angle = util::angle_ivec2(self.start_pos, target);

		let step_raw = util::step_angle(angle + PI / 2.0);
		let step_right_side = (step_raw * f32::from(size / 2)).as_ivec2();
		let step_left_side = -step_right_side;

		let cur_left = self.start_pos + step_left_side;
		let cur_right = self.start_pos + step_right_side;

		let next_left = target + step_left_side;
		let next_right = target + step_right_side;

		// first triangle
		for iter in TriangleRasterizerIter::iterate(cur_left, next_left, next_right) {
			self.affected_pixels.insert(iter.pos);
		}

		// second triangle
		for iter in TriangleRasterizerIter::iterate(cur_left, next_right, cur_right) {
			self.affected_pixels.insert(iter.pos);
		}

		let circle_shift = IVec2::splat(-(size as i32) / 2);

		let shape = BrushShape::new(&ShapeType::Circle, size, true);
		for pt in shape.iterate() {
			let shift = circle_shift + IVec2::new(i32::from(pt.local_x), i32::from(pt.local_y));
			self.affected_pixels.insert(self.start_pos + shift);
			self.affected_pixels.insert(target + shift);
		}

		// left edge
		self.write_line_iter(LineMoveIter::iterate(cur_left, next_left));

		// right edge
		self.write_line_iter(LineMoveIter::iterate(cur_right, next_right));
	}

	fn gen_pixels(&mut self, target: IVec2, size: u8) {
		if size <= 1 {
			self.gen_pixels_1px(target);
		} else {
			self.gen_pixels_thick(target, size);
		}
	}

	pub async fn process(
		&mut self,
		chunk_cache: &mut ChunkCache,
		refs: &RoomRefs,
		target: IVec2,
		color: ColorRGB,
		size: u8,
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

		// Clear previous iteration with transparent pixels
		let mut out_pixels = self.gen_global_pixel_vec_rgba(ColorRGBA::zero());
		self.affected_pixels.clear(); // generate line pixels from scratch

		self.gen_pixels(target, size);

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

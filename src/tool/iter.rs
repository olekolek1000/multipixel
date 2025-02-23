use glam::IVec2;

use crate::util;

pub struct LineMoveIter {
	index: u32,
	step: u8,
	pub iter_count: u32,
	start_x: i32,
	start_y: i32,
	end_x: i32,
	end_y: i32,
}

impl LineMoveIter {
	pub fn iterate(from: IVec2, to: IVec2, step: u8) -> Self {
		let iter_count = std::cmp::max(
			1,
			util::distance64(to.x as f64, to.y as f64, from.x as f64, from.y as f64).ceil() as u32,
		);

		Self {
			iter_count,
			start_x: from.x,
			start_y: from.y,
			end_x: to.x,
			end_y: to.y,
			index: 0,
			step,
		}
	}
}

pub struct LineMoveCell {
	pub pos: IVec2,
	pub index: u32,
}

impl Iterator for LineMoveIter {
	type Item = LineMoveCell;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index > self.iter_count {
			return None;
		}

		let alpha = (self.index as f64) / self.iter_count as f64;

		//Lerp
		let x = util::lerp(alpha, self.start_x as f64, self.end_x as f64) as i32;
		let y = util::lerp(alpha, self.start_y as f64, self.end_y as f64).round() as i32;

		let item = LineMoveCell {
			index: self.index,
			pos: IVec2::new(x, y),
		};

		self.index += self.step as u32;
		Some(item)
	}
}

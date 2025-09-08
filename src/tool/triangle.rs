use glam::{IVec2, UVec2};

pub struct TriangleRasterizerItem {
	pub pos: IVec2,
}

pub struct TriangleRasterizerIter {
	pt1_local: UVec2,
	pt2_local: UVec2,
	pt3_local: UVec2,

	shift: IVec2, // global top-left position
	size: UVec2,
	cur_local: UVec2,

	ended: bool,
}

const fn is_in_triangle(pt: IVec2, pt1: IVec2, pt2: IVec2, pt3: IVec2) -> bool {
	let d_x = pt.x - pt3.x;
	let d_y = pt.y - pt3.y;
	let d_x21 = pt3.x - pt2.x;
	let d_y12 = pt2.y - pt3.y;
	let d = d_y12 * (pt1.x - pt3.x) + d_x21 * (pt1.y - pt3.y);
	let s = d_y12 * d_x + d_x21 * d_y;
	let t = (pt3.y - pt1.y) * d_x + (pt1.x - pt3.x) * d_y;
	if d < 0 {
		return s <= 0 && t <= 0 && s + t >= d;
	}
	s >= 0 && t >= 0 && s + t <= d
}

// FIXME: this implementation is simple, slow and dumb
impl TriangleRasterizerIter {
	pub fn iterate(pt1: IVec2, pt2: IVec2, pt3: IVec2) -> Self {
		let boundary_left = pt1.x.min(pt2.x).min(pt3.x);
		let boundary_right = pt1.x.max(pt2.x).max(pt3.x);
		let boundary_top = pt1.y.min(pt2.y).min(pt3.y);
		let boundary_bottom = pt1.y.max(pt2.y).max(pt3.y);

		let shift = IVec2::new(boundary_left, boundary_top);

		Self {
			pt1_local: (pt1 - shift).as_uvec2(),
			pt2_local: (pt2 - shift).as_uvec2(),
			pt3_local: (pt3 - shift).as_uvec2(),
			shift,
			size: UVec2::new(
				(boundary_right - boundary_left) as u32,
				(boundary_bottom - boundary_top) as u32,
			),
			cur_local: UVec2::ZERO,
			ended: false,
		}
	}

	const fn step(&mut self) {
		self.cur_local.x += 1;
		if self.cur_local.x >= self.size.x {
			self.cur_local.x = 0;
			self.cur_local.y += 1;
			if self.cur_local.y >= self.size.y {
				self.ended = true;
			}
		}
	}
}

impl Iterator for TriangleRasterizerIter {
	type Item = TriangleRasterizerItem;

	fn next(&mut self) -> Option<Self::Item> {
		while !self.ended {
			if is_in_triangle(
				self.cur_local.as_ivec2(),
				self.pt1_local.as_ivec2(),
				self.pt2_local.as_ivec2(),
				self.pt3_local.as_ivec2(),
			) {
				let res = Some(TriangleRasterizerItem {
					pos: self.shift + self.cur_local.as_ivec2(),
				});
				self.step();
				return res;
			}
			self.step();
		}

		None
	}
}

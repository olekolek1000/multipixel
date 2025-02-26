use glam::IVec2;

use core::iter::Iterator;

pub struct LineIter {
	pub pos: IVec2,
}

pub struct LineMoveIter {
	pos: IVec2,
	dx: i32,
	dy: i32,
	x1: i32,
	diff: i32,
	octant: Octant,
}

struct Octant(u8);

impl Octant {
	fn points(start: IVec2, end: IVec2) -> Octant {
		let mut dx = end.x - start.x;
		let mut dy = end.y - start.y;

		let mut octant = 0;

		if dy < 0 {
			dx = -dx;
			dy = -dy;
			octant += 4;
		}

		if dx < 0 {
			let tmp = dx;
			dx = dy;
			dy = -tmp;
			octant += 2
		}

		if dx < dy {
			octant += 1
		}

		Octant(octant)
	}

	fn octant_to(&self, p: IVec2) -> IVec2 {
		match self.0 {
			0 => IVec2::new(p.x, p.y),
			1 => IVec2::new(p.y, p.x),
			2 => IVec2::new(p.y, -p.x),
			3 => IVec2::new(-p.x, p.y),
			4 => IVec2::new(-p.x, -p.y),
			5 => IVec2::new(-p.y, -p.x),
			6 => IVec2::new(-p.y, p.x),
			7 => IVec2::new(p.x, -p.y),
			_ => unreachable!(),
		}
	}

	fn octant_from(&self, p: IVec2) -> IVec2 {
		match self.0 {
			0 => IVec2::new(p.x, p.y),
			1 => IVec2::new(p.y, p.x),
			2 => IVec2::new(-p.y, p.x),
			3 => IVec2::new(-p.x, p.y),
			4 => IVec2::new(-p.x, -p.y),
			5 => IVec2::new(-p.y, -p.x),
			6 => IVec2::new(p.y, -p.x),
			7 => IVec2::new(p.x, -p.y),
			_ => unreachable!(),
		}
	}
}

impl LineMoveIter {
	pub fn iterate(start: IVec2, end: IVec2) -> LineMoveIter {
		let octant = Octant::points(start, end);
		let start = octant.octant_to(start);
		let end = octant.octant_to(end);
		let dx = end.x - start.x;
		let dy = end.y - start.y;

		LineMoveIter {
			pos: start,
			dx,
			dy,
			x1: end.x,
			diff: dy - dx,
			octant,
		}
	}
}

impl Iterator for LineMoveIter {
	type Item = LineIter;

	fn next(&mut self) -> Option<Self::Item> {
		if self.pos.x > self.x1 {
			return None;
		}

		let p = self.pos;

		if self.diff >= 0 {
			self.pos.y += 1;
			self.diff -= self.dx;
		}

		self.diff += self.dy;

		self.pos.x += 1;

		Some(LineIter {
			pos: self.octant.octant_from(p),
		})
	}
}

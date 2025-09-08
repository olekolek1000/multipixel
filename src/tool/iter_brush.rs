use std::{collections::HashMap, sync::Arc};

pub struct BrushShape {
	pub size: u8, // Width and height
	pub data: Vec<u8>,
}

pub struct BrushShapeIterCell {
	pub local_x: u8,
	pub local_y: u8,
}

pub struct BrushShapeIter<'a> {
	shape: &'a BrushShape,
	cur_x: u8,
	cur_y: u8,
}

impl BrushShapeIter<'_> {
	const fn go_next(&mut self) {
		self.cur_x += 1;
		if self.cur_x >= self.shape.size {
			self.cur_x = 0;
			self.cur_y += 1;
		}
	}
}

impl Iterator for BrushShapeIter<'_> {
	type Item = BrushShapeIterCell;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if self.cur_y >= self.shape.size {
				return None;
			}

			unsafe {
				if *self.shape.data.get_unchecked(
					(u32::from(self.cur_y) * u32::from(self.shape.size) + u32::from(self.cur_x)) as usize,
				) == 1
				{
					let cell = BrushShapeIterCell {
						local_x: self.cur_x,
						local_y: self.cur_y,
					};
					self.go_next();
					return Some(cell);
				}
			}

			self.go_next();
		}
	}
}

pub enum ShapeType {
	Circle,
	Square,
}

impl BrushShape {
	pub fn new(shape_type: &ShapeType, size: u8, filled: bool) -> Self {
		let data = match shape_type {
			ShapeType::Circle => (0..size)
				.flat_map(|y| (0..size).map(move |x| (x, y)))
				.map(|(x, y)| {
					let diff_x = (i32::from(size) / 2) - i32::from(x);
					let diff_y = (i32::from(size) / 2) - i32::from(y);
					let distance = ((diff_x * diff_x + diff_y * diff_y) as f32).sqrt();

					if filled {
						u8::from(distance <= (f32::from(size) - 0.1) / 2.0)
					} else {
						u8::from(
							distance <= (f32::from(size) - 0.1) / 2.0
								&& distance >= (f32::from(size) / 2.0) - 2.0,
						)
					}
				})
				.collect::<Vec<u8>>(),
			ShapeType::Square => (0..size)
				.flat_map(|y| (0..size).map(move |x| (x, y)))
				.map(|(x, y)| {
					if filled {
						1 // kek
					} else {
						u8::from(x == 0 || y == 0 || x == size - 1 || y == size - 1)
					}
				})
				.collect::<Vec<u8>>(),
		};

		Self { size, data }
	}

	pub const fn iterate(&self) -> BrushShapeIter {
		BrushShapeIter {
			shape: self,
			cur_x: 0,
			cur_y: 0,
		}
	}
}

pub struct BrushShapes {
	circle_filled: HashMap<u8, Arc<BrushShape>>,
	circle_outline: HashMap<u8, Arc<BrushShape>>,

	square_filled: HashMap<u8, Arc<BrushShape>>,
	square_outline: HashMap<u8, Arc<BrushShape>>,
}

impl BrushShapes {
	pub fn new() -> Self {
		Self {
			circle_filled: HashMap::new(),
			circle_outline: HashMap::new(),

			square_filled: HashMap::new(),
			square_outline: HashMap::new(),
		}
	}

	fn get(
		shape_type: &ShapeType,
		map: &mut HashMap<u8, Arc<BrushShape>>,
		size: u8,
	) -> Arc<BrushShape> {
		if let Some(shape) = map.get(&size) {
			return shape.clone();
		}
		let shape = Arc::new(BrushShape::new(shape_type, size, true));
		map.insert(size, shape.clone());
		shape
	}

	pub fn get_square_filled(&mut self, size: u8) -> Arc<BrushShape> {
		Self::get(&ShapeType::Square, &mut self.square_filled, size)
	}

	pub fn get_square_outline(&mut self, size: u8) -> Arc<BrushShape> {
		Self::get(&ShapeType::Square, &mut self.square_outline, size)
	}

	pub fn get_circle_filled(&mut self, size: u8) -> Arc<BrushShape> {
		Self::get(&ShapeType::Circle, &mut self.circle_filled, size)
	}

	pub fn get_circle_outline(&mut self, size: u8) -> Arc<BrushShape> {
		Self::get(&ShapeType::Circle, &mut self.circle_outline, size)
	}
}

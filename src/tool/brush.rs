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
	fn go_next(&mut self) {
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
				if *self
					.shape
					.data
					.get_unchecked((self.cur_y as u32 * self.shape.size as u32 + self.cur_x as u32) as usize)
					== 1
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
	pub fn new(shape_type: ShapeType, size: u8, filled: bool) -> Self {
		let data = match shape_type {
			ShapeType::Circle => (0..size)
				.flat_map(|y| (0..size).map(move |x| (x, y)))
				.map(|(x, y)| {
					let diff_x = (size as i32 / 2) - x as i32;
					let diff_y = (size as i32 / 2) - y as i32;
					let distance = ((diff_x * diff_x + diff_y * diff_y) as f32).sqrt();

					if filled {
						(distance <= (size as f32) / 2.0) as u8
					} else {
						(distance <= (size as f32) / 2.0 && distance >= (size as f32 / 2.0) - 2.0) as u8
					}
				})
				.collect::<Vec<u8>>(),
			ShapeType::Square => (0..size)
				.flat_map(|y| (0..size).map(move |x| (x, y)))
				.map(|(x, y)| {
					if filled {
						1 // kek
					} else {
						(x == 0 || y == 0 || x == size - 1 || y == size - 1) as u8
					}
				})
				.collect::<Vec<u8>>(),
		};

		Self { size, data }
	}

	pub fn iterate(&self) -> BrushShapeIter {
		BrushShapeIter {
			shape: self,
			cur_x: 0,
			cur_y: 0,
		}
	}
}

pub struct BrushShapes {
	shapes_circle_filled: HashMap<u8, Arc<BrushShape>>,
	shapes_circle_outline: HashMap<u8, Arc<BrushShape>>,

	shapes_square_filled: HashMap<u8, Arc<BrushShape>>,
	shapes_square_outline: HashMap<u8, Arc<BrushShape>>,
}

impl BrushShapes {
	pub fn new() -> Self {
		Self {
			shapes_circle_filled: HashMap::new(),
			shapes_circle_outline: HashMap::new(),

			shapes_square_filled: HashMap::new(),
			shapes_square_outline: HashMap::new(),
		}
	}

	fn get(
		shape_type: ShapeType,
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
		BrushShapes::get(ShapeType::Square, &mut self.shapes_square_filled, size)
	}

	pub fn get_square_outline(&mut self, size: u8) -> Arc<BrushShape> {
		BrushShapes::get(ShapeType::Square, &mut self.shapes_square_outline, size)
	}

	pub fn get_circle_filled(&mut self, size: u8) -> Arc<BrushShape> {
		BrushShapes::get(ShapeType::Circle, &mut self.shapes_circle_filled, size)
	}

	pub fn get_circle_outline(&mut self, size: u8) -> Arc<BrushShape> {
		BrushShapes::get(ShapeType::Circle, &mut self.shapes_circle_outline, size)
	}
}

use std::{collections::HashMap, sync::Arc};

pub struct BrushShape {
	pub size: u8, // Width and height
	pub data: Vec<u8>,
}

impl BrushShape {
	pub fn new(size: u8, filled: bool) -> Self {
		let capacity = size as usize * size as usize;
		let mut data: Vec<u8> = Vec::with_capacity(capacity);

		unsafe {
			data.set_len(capacity);
			// Generate circle

			let center_x = (size / 2) as i32;
			let center_y = (size / 2) as i32;

			for y in 0..size {
				for x in 0..size {
					let diff_x = center_x - x as i32;
					let diff_y = center_y - y as i32;
					let distance = ((diff_x * diff_x + diff_y * diff_y) as f32).sqrt();
					let index = y as usize * size as usize + x as usize;

					if filled {
						*data.get_unchecked_mut(index) = (distance <= (size as f32) / 2.0).into();
					} else {
						*data.get_unchecked_mut(index) =
							(distance <= (size as f32) / 2.0 && distance >= (size as f32 / 2.0) - 2.0).into();
					}
				}
			}
		}

		Self { size, data }
	}
}

pub struct BrushShapes {
	shapes_circle_filled: HashMap<u8, Arc<BrushShape>>,
	shapes_circle_outline: HashMap<u8, Arc<BrushShape>>,
}

impl BrushShapes {
	pub fn new() -> Self {
		Self {
			shapes_circle_filled: HashMap::new(),
			shapes_circle_outline: HashMap::new(),
		}
	}

	fn get(map: &mut HashMap<u8, Arc<BrushShape>>, size: u8) -> Arc<BrushShape> {
		if let Some(shape) = map.get(&size) {
			return shape.clone();
		}
		let shape = Arc::new(BrushShape::new(size, true));
		map.insert(size, shape.clone());
		shape
	}

	pub fn get_filled(&mut self, size: u8) -> Arc<BrushShape> {
		BrushShapes::get(&mut self.shapes_circle_filled, size)
	}

	pub fn get_outline(&mut self, size: u8) -> Arc<BrushShape> {
		BrushShapes::get(&mut self.shapes_circle_outline, size)
	}
}

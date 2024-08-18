use std::{collections::HashMap, sync::Arc};

pub struct BrushShape {
	pub size: u8, // Width and height
	pub data: Vec<u8>,
}

impl BrushShape {
	pub fn new(size: u8, filled: bool) -> Self {
		let data = (0..size)
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
			.collect::<Vec<u8>>();

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

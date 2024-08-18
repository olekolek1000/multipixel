use glam::IVec2;

#[derive(Default, Clone)]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

#[derive(Default)]
pub struct GlobalPixel {
	pub pos: IVec2,
	pub color: Color,
}

impl GlobalPixel {
	pub fn insert_to_vec(vec: &mut Vec<GlobalPixel>, x: i32, y: i32, color: &Color) {
		vec.push(GlobalPixel {
			pos: IVec2::new(x, y),
			color: color.clone(),
		})
	}
}

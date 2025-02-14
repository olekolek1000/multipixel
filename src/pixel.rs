use glam::IVec2;

#[derive(Default, Clone, PartialEq)]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

impl Color {
	pub fn blend(alpha: u8, from: &Color, to: &Color) -> Color {
		Color {
			r: ((to.r as u16 * alpha as u16 + from.r as u16 * (255 - alpha) as u16) / 255) as u8,
			g: ((to.g as u16 * alpha as u16 + from.g as u16 * (255 - alpha) as u16) / 255) as u8,
			b: ((to.b as u16 * alpha as u16 + from.b as u16 * (255 - alpha) as u16) / 255) as u8,
		}
	}
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

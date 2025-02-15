use glam::IVec2;

#[derive(Default, Clone, PartialEq)]
pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

impl Color {
	pub fn blend_gamma_corrected(alpha: u8, from: &Color, to: &Color) -> Color {
		let alpha = alpha as u32;
		let alpha_inv = 255 - alpha;
		let from_r = from.r as u32;
		let from_g = from.g as u32;
		let from_b = from.b as u32;
		let to_r = to.r as u32;
		let to_g = to.g as u32;
		let to_b = to.b as u32;
		// sqrt() is implemented at hardware level for floats in ALU
		Color {
			r: f32::sqrt((((from_r * from_r) * alpha_inv + (to_r * to_r) * alpha) / 255) as f32) as u8,
			g: f32::sqrt((((from_g * from_g) * alpha_inv + (to_g * to_g) * alpha) / 255) as f32) as u8,
			b: f32::sqrt((((from_b * from_b) * alpha_inv + (to_b * to_b) * alpha) / 255) as f32) as u8,
		}
	}

	#[allow(dead_code)]
	pub fn blend_raw(alpha: u8, from: &Color, to: &Color) -> Color {
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

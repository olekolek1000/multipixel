use glam::IVec2;

#[derive(Default, Copy, Clone, PartialEq)]
pub struct ColorRGB {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

impl ColorRGB {
	pub fn rgba(&self, a: u8) -> ColorRGBA {
		ColorRGBA {
			r: self.r,
			g: self.g,
			b: self.b,
			a,
		}
	}
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct ColorRGBA {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

impl ColorRGBA {
	pub fn zero() -> ColorRGBA {
		ColorRGBA {
			r: 0,
			g: 0,
			b: 0,
			a: 0,
		}
	}

	pub fn blend_gamma_corrected(alpha: u8, from: &ColorRGBA, to: &ColorRGBA) -> ColorRGBA {
		let alpha = alpha as u32;
		let alpha_inv = 255 - alpha;
		let from_r = from.r as u32;
		let from_g = from.g as u32;
		let from_b = from.b as u32;
		let from_a = from.a as u32;
		let to_r = to.r as u32;
		let to_g = to.g as u32;
		let to_b = to.b as u32;
		let to_a = to.a as u32;
		// sqrt() is implemented at hardware level for floats in ALU
		ColorRGBA {
			r: f32::sqrt((((from_r * from_r) * alpha_inv + (to_r * to_r) * alpha) / 255) as f32) as u8,
			g: f32::sqrt((((from_g * from_g) * alpha_inv + (to_g * to_g) * alpha) / 255) as f32) as u8,
			b: f32::sqrt((((from_b * from_b) * alpha_inv + (to_b * to_b) * alpha) / 255) as f32) as u8,
			a: f32::sqrt((((from_a * from_a) * alpha_inv + (to_a * to_a) * alpha) / 255) as f32) as u8,
		}
	}
}

#[derive(Default)]
pub struct GlobalPixelRGBA {
	pub pos: IVec2,
	pub color: ColorRGBA,
}

impl GlobalPixelRGBA {
	pub fn insert_to_vec(vec: &mut Vec<GlobalPixelRGBA>, x: i32, y: i32, color: &ColorRGBA) {
		vec.push(GlobalPixelRGBA {
			pos: IVec2::new(x, y),
			color: *color,
		})
	}
}

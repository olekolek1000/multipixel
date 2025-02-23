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

impl ColorRGB {
	pub fn blend_gamma_corrected(alpha: u8, from: &ColorRGB, to: &ColorRGB) -> ColorRGB {
		let alpha = alpha as u32;
		let alpha_inv = 255 - alpha;
		let from_r = from.r as u32;
		let from_g = from.g as u32;
		let from_b = from.b as u32;
		let to_r = to.r as u32;
		let to_g = to.g as u32;
		let to_b = to.b as u32;
		// sqrt() is implemented at hardware level for floats in ALU
		ColorRGB {
			r: f32::sqrt((((from_r * from_r) * alpha_inv + (to_r * to_r) * alpha) / 255) as f32) as u8,
			g: f32::sqrt((((from_g * from_g) * alpha_inv + (to_g * to_g) * alpha) / 255) as f32) as u8,
			b: f32::sqrt((((from_b * from_b) * alpha_inv + (to_b * to_b) * alpha) / 255) as f32) as u8,
		}
	}
}

impl ColorRGBA {
	pub fn rgb(&self) -> ColorRGB {
		ColorRGB {
			r: self.r,
			g: self.g,
			b: self.b,
		}
	}

	pub fn zero() -> ColorRGBA {
		ColorRGBA {
			r: 0,
			g: 0,
			b: 0,
			a: 0,
		}
	}
}

#[derive(Default)]
pub struct GlobalPixelRGB {
	pub pos: IVec2,
	pub color: ColorRGB,
}

#[derive(Default)]
pub struct GlobalPixelRGBA {
	pub pos: IVec2,
	pub color: ColorRGBA,
}

impl GlobalPixelRGB {
	pub fn insert_to_vec(vec: &mut Vec<GlobalPixelRGB>, x: i32, y: i32, color: &ColorRGB) {
		vec.push(GlobalPixelRGB {
			pos: IVec2::new(x, y),
			color: *color,
		})
	}
}

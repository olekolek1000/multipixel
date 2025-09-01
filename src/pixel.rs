use glam::IVec2;

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct ColorRGB {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

impl ColorRGB {
	pub const fn rgba(self, a: u8) -> ColorRGBA {
		ColorRGBA {
			r: self.r,
			g: self.g,
			b: self.b,
			a,
		}
	}
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct ColorRGBA {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

impl ColorRGBA {
	pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
		Self { r, g, b, a }
	}

	pub const fn zero() -> Self {
		Self {
			r: 0,
			g: 0,
			b: 0,
			a: 0,
		}
	}

	pub fn blend_gamma_corrected(alpha: u8, from: Self, to: Self) -> Self {
		let alpha = u32::from(alpha);
		let alpha_inv = 255 - alpha;
		let from_r = u32::from(from.r);
		let from_g = u32::from(from.g);
		let from_b = u32::from(from.b);
		let from_a = u32::from(from.a);
		let to_r = u32::from(to.r);
		let to_g = u32::from(to.g);
		let to_b = u32::from(to.b);
		let to_a = u32::from(to.a);
		// sqrt() is implemented at hardware level for floats in ALU
		Self {
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
	pub fn insert_to_vec(vec: &mut Vec<Self>, x: i32, y: i32, color: ColorRGBA) {
		vec.push(Self {
			pos: IVec2::new(x, y),
			color,
		});
	}
}

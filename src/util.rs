use glam::IVec2;

pub fn distance_squared_int32(from: IVec2, to: IVec2) -> i32 {
	i32::abs(from.x - to.x) + i32::abs(from.y - to.y)
}

pub fn distance32(p1_x: f32, p1_y: f32, p2_x: f32, p2_y: f32) -> f32 {
	f32::sqrt(f32::powf(p1_x - p2_x, 2.0) + f32::powf(p1_y - p2_y, 2.0))
}

pub fn distance64(p1_x: f64, p1_y: f64, p2_x: f64, p2_y: f64) -> f64 {
	f64::sqrt(f64::powf(p1_x - p2_x, 2.0) + f64::powf(p1_y - p2_y, 2.0))
}

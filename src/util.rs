pub fn distance(p1_x: f32, p1_y: f32, p2_x: f32, p2_y: f32) -> f32 {
	f32::sqrt(f32::powf(p1_x - p2_x, 2.0) + f32::powf(p1_y - p2_y, 2.0))
}

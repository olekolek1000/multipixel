pub fn distance(p1_x: f64, p1_y: f64, p2_x: f64, p2_y: f64) -> f64 {
	f64::sqrt(f64::powf(p1_x - p2_x, 2.0) + f64::powf(p1_y - p2_y, 2.0))
}

pub fn lerp(alpha: f64, prev: f64, var: f64) -> f64 {
	var * alpha + prev * (1.0 - alpha)
}

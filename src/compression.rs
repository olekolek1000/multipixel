pub fn compress_lz4(raw: &[u8]) -> Vec<u8> {
	lz4_flex::block::compress(raw)
}

pub fn decompress_lz4(compressed: &[u8], min_size: usize) -> Option<Vec<u8>> {
	match lz4_flex::block::decompress(compressed, min_size) {
		Ok(data) => Some(data),
		Err(e) => {
			log::error!("Cannot decompress lz4 data: {e}");
			None
		}
	}
}

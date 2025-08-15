use std::collections::HashMap;

use glam::U8Vec2;
use smallvec::SmallVec;

use super::layer::LayerRGBA;
use crate::{limits::CHUNK_SIZE_PX, pixel::ColorRGBA, session::SessionHandle};

pub struct CompositionLayer {
	pub layer: LayerRGBA,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum LayerID {
	Session(u64, SessionHandle),
}

pub struct Compositor {
	layers: HashMap<LayerID, CompositionLayer>,
}

impl CompositionLayer {
	pub fn new() -> Self {
		Self {
			layer: LayerRGBA::new(),
		}
	}
}

// for debugging
const SHOW_FOR_ALL: bool = true;

impl Compositor {
	pub fn new() -> Self {
		Self {
			layers: HashMap::new(),
		}
	}

	pub fn get_or_alloc_mut(&mut self, id: &LayerID) -> &mut CompositionLayer {
		self
			.layers
			.entry(id.clone())
			.or_insert_with(CompositionLayer::new)
	}

	pub fn get(&self, layer_id: &LayerID) -> Option<&CompositionLayer> {
		self
			.layers
			.iter()
			.find(|l| *l.0 == *layer_id)
			.map(|pair| pair.1)
	}

	#[allow(dead_code)]
	pub fn get_mut(&mut self, layer_id: &LayerID) -> Option<&mut CompositionLayer> {
		self
			.layers
			.iter_mut()
			.find(|l| *l.0 == *layer_id)
			.map(|pair| pair.1)
	}

	pub fn calc_pixel(base: &LayerRGBA, layers: &[&LayerRGBA], chunk_pixel_pos: U8Vec2) -> ColorRGBA {
		debug_assert!(base.read().is_some());

		let mut col = base.get_pixel(chunk_pixel_pos);

		for layer in layers {
			debug_assert!(layer.read().is_some());
			let rgba = layer.get_pixel(chunk_pixel_pos);

			col = ColorRGBA::blend_gamma_corrected(rgba.a, &col, &rgba);
		}

		col
	}

	pub fn remove_layer_id(&mut self, layer_id: LayerID) {
		self.layers.retain(|id, _| *id != layer_id);
	}

	pub fn deref_session(&mut self, handle: &SessionHandle) {
		self.layers.retain(|id, _| match id {
			LayerID::Session(_, hnd) => hnd != handle,
		});
	}

	pub fn has_session_composition(&self, handle: &SessionHandle) -> bool {
		if SHOW_FOR_ALL {
			return true;
		}
		self.layers.iter().any(|(layer_id, _)| match layer_id {
			LayerID::Session(_, hnd) => handle == hnd,
		})
	}

	pub fn construct_layers_from_session(&self, handle: &SessionHandle) -> SmallVec<[&LayerRGBA; 4]> {
		let mut out = SmallVec::<[&LayerRGBA; 4]>::new();

		for (layer_id, layer) in &self.layers {
			match layer_id {
				LayerID::Session(_gen, hnd) => {
					if *handle == *hnd || SHOW_FOR_ALL {
						out.push(&layer.layer);
					}
				}
			}
		}

		out
	}

	pub fn composite(base: &LayerRGBA, layers: &[&LayerRGBA]) -> LayerRGBA {
		debug_assert!(base.read().is_some());
		for layer in layers {
			debug_assert!(layer.read().is_some())
		}

		let mut out = LayerRGBA::new();

		// Fill with base background data
		out.set_data(base.read_unchecked().clone());

		//it's safe, trust me
		unsafe {
			// Composite layer by layer
			for layer in layers {
				debug_assert!(layer.read().is_some());
				let out_rgba = out.read_unchecked_mut().0.as_mut_ptr();
				let layer_rgba = layer.read_unchecked();

				for y in 0..CHUNK_SIZE_PX {
					for x in 0..CHUNK_SIZE_PX {
						let offset_rgba = (y * (CHUNK_SIZE_PX * 4) + x * 4) as isize;

						let out_red = out_rgba.offset(offset_rgba);
						let out_green = out_rgba.offset(offset_rgba + 1);
						let out_blue = out_rgba.offset(offset_rgba + 2);
						let out_alpha = out_rgba.offset(offset_rgba + 3);

						let layer_red = (layer_rgba.0)[(offset_rgba) as usize];
						let layer_green = (layer_rgba.0)[(offset_rgba + 1) as usize];
						let layer_blue = (layer_rgba.0)[(offset_rgba + 2) as usize];
						let layer_alpha = (layer_rgba.0)[(offset_rgba + 3) as usize];

						let blended = ColorRGBA::blend_gamma_corrected(
							layer_alpha,
							&ColorRGBA {
								r: *out_red,
								g: *out_green,
								b: *out_blue,
								a: *out_alpha,
							},
							&ColorRGBA {
								r: layer_red,
								g: layer_green,
								b: layer_blue,
								a: layer_alpha,
							},
						);

						*out_red = blended.r;
						*out_green = blended.g;
						*out_blue = blended.b;
						*out_alpha = blended.a;
					}
				}
			}
		}

		out
	}
}

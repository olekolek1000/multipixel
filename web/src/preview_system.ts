import { CHUNK_SIZE, type PreviewBoundary } from "./chunk_map";
import { easings } from "./easings";
import { RenderEngine, Texture } from "./render_engine";
import type { RoomInstance } from "./room_instance";

export class Preview {
	x: number;
	y: number;
	gl: WebGL2RenderingContext;
	remove_timeout: number = 0;

	tex: Texture | null = null;
	tex_creation_time_millis: number = 0;
	fading_in: boolean = false;

	constructor(gl: WebGL2RenderingContext, x: number, y: number) {
		this.x = x;
		this.y = y;
		this.gl = gl;
	}

	setData(rgba: Uint8Array) {
		if (!this.tex) {
			this.tex = new Texture();
			this.tex.texture = this.gl.createTexture()!;
		}

		let gl = this.gl;

		gl.bindTexture(gl.TEXTURE_2D, this.tex.texture);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, CHUNK_SIZE, CHUNK_SIZE, 0, gl.RGBA, gl.UNSIGNED_BYTE, rgba);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

		this.tex_creation_time_millis = (new Date()).getTime();
		this.fading_in = true;
	}

	render(renderer: RenderEngine, size: number, cur_time_millis: number): boolean {
		if (this.tex === null) {
			return false;
		}

		const x = this.x * size;
		const y = this.y * size;

		if (this.fading_in) {
			const lifetime = cur_time_millis - this.tex_creation_time_millis;
			const duration = 500.0;
			const alpha = easings.out_cubic(lifetime / duration);

			renderer.shader_colored.setColor(renderer.gl, 1.0, 1.0, 1.0, alpha);
			renderer.drawRect(renderer.shader_colored, this.tex, x, y, size, size);

			if (lifetime > duration) {
				this.fading_in = false;
			}
		}
		else {
			renderer.drawRect(renderer.shader_solid, this.tex, x, y, size, size);
		}

		return this.fading_in;
	}

	destructor(gl: WebGL2RenderingContext) {
		if (this.tex)
			gl.deleteTexture(this.tex.texture);
	}
}

export class PreviewLayer {
	system: PreviewSystem;
	zoom: number;
	previews = new Map<number, Map<number, Preview>>();

	constructor(system: PreviewSystem, zoom: number) {
		this.system = system;
		this.zoom = zoom;
	}

	getPreview(x: number, y: number): Preview | null {
		let mx = this.previews.get(x);
		if (!mx) return null;
		let preview = mx.get(y);
		return preview ? preview : null;
	}

	iterPreviewsInBoundary(boundary: PreviewBoundary, func: (preview: Preview) => void) {
		for (const [x, mx] of this.previews) {
			if (x < boundary.start_x || x > boundary.end_x) {
				continue;
			}

			for (const [y, preview] of mx) {
				if (y < boundary.start_y || y > boundary.end_y) {
					continue;
				}

				func(preview);
			}
		}
	}

	getOrCreatePreview(x: number, y: number) {
		let mx = this.previews.get(x);
		if (!mx) {
			mx = new Map<number, Preview>();
			this.previews.set(x, mx);
		}

		let preview = mx.get(y);
		if (preview) {
			return preview;
		}

		preview = new Preview(this.system.instance.state!.renderer.gl, x, y);
		mx.set(y, preview);

		//console.log("created preview at " + x + ", " + y);

		return preview;
	}

	removePreview(x: number, y: number) {
		let mx = this.previews.get(x);
		if (!mx)
			return;//Not found

		let chunk = mx.get(y);
		if (!chunk)
			return;//Not found

		//Remove preview
		chunk.destructor(this.system.instance.state!.renderer.gl);
		mx.delete(y);
	}
}

export class PreviewSystem {
	instance: RoomInstance;
	layers = new Map<number /*zoom*/, PreviewLayer>();

	constructor(instance: RoomInstance) {
		this.instance = instance;
	}

	getOrCreateLayer(zoom: number) {
		let layer = this.layers.get(zoom);
		if (layer)
			return layer;

		layer = new PreviewLayer(this, zoom);
		this.layers.set(zoom, layer);
		return layer;
	}

	getLayer(zoom: number) {
		let layer = this.layers.get(zoom);
		return layer ? layer : null;
	}
}
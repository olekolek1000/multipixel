import { User } from "./client";
import { easings } from "./easings";
import { RenderEngine, Texture } from "./render_engine";
import { ConnectedInstanceState, PREVIEW_SYSTEM_LAYER_COUNT, type RoomInstance } from "./room_instance";

export const CHUNK_SIZE = 256;

class PixelQueueCell {
	x: number;
	y: number;
	red: number;
	green: number;
	blue: number;
	alpha: number;

	constructor(x: number, y: number, red: number, green: number, blue: number, alpha: number) {
		this.x = x;
		this.y = y;
		this.red = red;
		this.green = green;
		this.blue = blue;
		this.alpha = alpha;
	}
}

class Chunk {
	x: number; // X position
	y: number; // Y position
	tex: Texture | null = null;
	tex_creation_time_millis: number = 0;
	fading_in: boolean = false;
	pixels: Uint8Array | null = null;

	pixel_queue: Array<PixelQueueCell> = [];

	constructor(_gl: WebGL2RenderingContext, x: number, y: number) {
		this.x = x;
		this.y = y;
	}

	initTexture(gl: WebGL2RenderingContext) {
		if (this.tex) return;//Already initialized

		this.tex = new Texture();
		this.tex.texture = gl.createTexture()!;
		gl.bindTexture(gl.TEXTURE_2D, this.tex.texture);

		this.pixels = new Uint8Array(CHUNK_SIZE * CHUNK_SIZE * 4);
		this.updateTexture(gl);

		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

		this.tex_creation_time_millis = (new Date()).getTime();
		this.fading_in = true;
	}

	destructor(gl: WebGL2RenderingContext) {
		if (this.tex) {
			gl.deleteTexture(this.tex.texture);
		}
		this.pixels = null;
	}

	updateTexture(gl: WebGL2RenderingContext) {
		this.initTexture(gl);
		gl.bindTexture(gl.TEXTURE_2D, this.tex!.texture);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, CHUNK_SIZE, CHUNK_SIZE, 0, gl.RGBA, gl.UNSIGNED_BYTE, this.pixels);
	}

	putPixel(x: number, y: number, red: number, green: number, blue: number, alpha: number) {
		this.pixel_queue.push(new PixelQueueCell(x, y, red, green, blue, alpha));
	}

	// returns array of 4 numbers (R,G,B,A)
	getPixel(x: number, y: number) {
		let data = this.pixels!;
		let image_offset = y * CHUNK_SIZE * 4 + x * 4;
		return [
			data[image_offset + 0],
			data[image_offset + 1],
			data[image_offset + 2],
			data[image_offset + 3]
		];
	}

	putImage(gl: WebGL2RenderingContext, dataview_rgba: DataView) {
		this.initTexture(gl);
		let offset = 0;
		let data = this.pixels!;

		for (let y = 0; y < CHUNK_SIZE; y++) {
			for (let x = 0; x < CHUNK_SIZE; x++) {
				let red = dataview_rgba.getUint8(offset + 0);
				let green = dataview_rgba.getUint8(offset + 1);
				let blue = dataview_rgba.getUint8(offset + 2);
				let alpha = dataview_rgba.getUint8(offset + 3);
				offset += 4;

				let image_offset = y * CHUNK_SIZE * 4 + x * 4;
				data[image_offset + 0] = red;
				data[image_offset + 1] = green;
				data[image_offset + 2] = blue;
				data[image_offset + 3] = alpha;
			}
		}

		gl.bindTexture(gl.TEXTURE_2D, this.tex!.texture);
		this.updateTexture(gl);
	}

	processPixels(gl: WebGL2RenderingContext) {
		if (this.tex === null) {
			return;
		}

		let count = this.pixel_queue.length;

		if (count == 0) {
			return false;
		}

		let data = this.pixels!;

		for (let i = 0; i < count; i++) {
			let cell = this.pixel_queue[i];

			let offset = cell.y * CHUNK_SIZE * 4 + cell.x * 4;
			data[offset + 0] = cell.red;
			data[offset + 1] = cell.green;
			data[offset + 2] = cell.blue;
			data[offset + 3] = cell.alpha;
		}

		this.updateTexture(gl);
		this.pixel_queue = [];

		return true;
	}

	render(renderer: RenderEngine, cur_time_millis: number): boolean {
		if (this.tex === null) {
			return false;
		}

		const x = this.x * CHUNK_SIZE;
		const y = this.y * CHUNK_SIZE;
		const width = CHUNK_SIZE;
		const height = CHUNK_SIZE;

		if (this.fading_in) {
			const lifetime = cur_time_millis - this.tex_creation_time_millis;
			const duration = 500.0;
			const alpha = easings.out_cubic(lifetime / duration);

			renderer.shader_colored.setColor(renderer.gl, 1.0, 1.0, 1.0, alpha);
			renderer.drawRect(renderer.shader_colored, this.tex, x, y, width, height);

			if (lifetime > duration) {
				this.fading_in = false;
			}
		}
		else {
			renderer.enableBlending(false);
			renderer.drawRect(renderer.shader_solid, this.tex, x, y, width, height);
			renderer.enableBlending(true);
		}


		return this.fading_in;
	}
}

class TextCacheCell {
	tex = new Texture();
	processed = false;
	width = 0;
	height = 0;
}

//Viewport boundary
export class Boundary {
	center_x: number = 0.0;
	center_y: number = 0.0;
	width: number = 0.0;
	height: number = 0.0;
};

export interface PreviewBoundary {
	start_x: number;
	start_y: number;
	end_x: number;
	end_y: number;
}

export class ChunkMap {
	instance: RoomInstance;
	state: ConnectedInstanceState;
	needs_redraw: boolean = true;
	texture_cursor!: Texture;
	texture_brush!: Texture;
	boundary: Boundary = new Boundary();
	text_cache = new Map<string, TextCacheCell>();
	map = new Map<number, Map<number, Chunk>>();

	scrolling = {
		x: 0,
		y: 0,
		zoom: 1.0,
	}

	constructor(instance: RoomInstance, state: ConnectedInstanceState) {
		this.instance = instance;
		this.state = state;
		let renderer = state.renderer;

		renderer.loadTextureImage("img/cursor.png", (tex: Texture) => {
			this.texture_cursor = tex;
		});

		renderer.loadTextureImage("img/brush.png", (tex: Texture) => {
			this.texture_brush = tex;
		});

		window.addEventListener("resize", () => {
			this.resize();
		});

		this.resize();
		this.updateBoundary();
	}

	textCacheGet(gl: WebGL2RenderingContext, text: string) {
		let cell = this.text_cache.get(text);
		if (!cell) {
			cell = new TextCacheCell();
			this.text_cache.set(text, cell);
		}

		if (cell.processed) {
			return cell;
		}

		cell.processed = true;

		let canvas = document.createElement("canvas");
		canvas.width = 256;
		canvas.height = 24;
		let ctx = canvas.getContext("2d")!;

		ctx.font = "16px Helvetica";
		for (let y = -1; y <= 1; y++) {
			for (let x = -1; x <= 1; x++) {
				if (x == 0 && y == 0) continue;
				ctx.fillStyle = "#000000";
				ctx.fillText(text, 0 + x, 16 + y);
			}
		}

		ctx.fillStyle = "#FFFFFF";
		ctx.fillText(text, 0, 16);

		let dim = ctx.measureText(text);

		cell.tex.texture = gl.createTexture()!;
		gl.bindTexture(gl.TEXTURE_2D, cell.tex.texture);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, dim.width, canvas.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
		gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, dim.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, canvas);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

		cell.width = dim.width;
		cell.height = canvas.height;

		return cell;
	}

	resize() {
		let renderer = this.state.renderer;
		let canvas = renderer.params.canvas;

		renderer.display_scale = window.devicePixelRatio || 1;

		canvas.width = window.innerWidth * renderer.display_scale;
		canvas.height = window.innerHeight * renderer.display_scale;

		//console.log(renderer.display_scale, canvas.width, canvas.height);
		this.triggerRerender();
	}

	triggerRerender() {
		this.needs_redraw = true;
	}

	chunkExists(x: number, y: number) {
		let mx = this.map.get(x);
		if (!mx)
			return false;

		let my = mx.get(y);
		return !!my;
	}

	getChunk(x: number, y: number) {
		let mx = this.map.get(x);
		if (!mx)
			return null;
		return mx.get(y);
	}

	iterChunksInBoundary(boundary: PreviewBoundary, func: (preview: Chunk) => void) {
		for (const [x, mx] of this.map) {
			if (x < boundary.start_x || x > boundary.end_x) {
				continue;
			}

			for (const [y, chunk] of mx) {
				if (y < boundary.start_y || y > boundary.end_y) {
					continue;
				}

				func(chunk);
			}
		}
	}


	createChunk(x: number, y: number) {
		let mx = this.map.get(x);
		if (!mx) {//Create row if not exists
			mx = new Map<number, Chunk>();
			this.map.set(x, mx);
		}

		let existing_chunk = this.getChunk(x, y);
		if (existing_chunk) {
			//Chunk already created, return existing
			return existing_chunk;
		}

		let new_chunk = new Chunk(this.state.renderer.gl, x, y);
		mx.set(y, new_chunk);

		//console.log("Created chunk at ", x, y);
		return new_chunk;
	}

	removeChunk(x: number, y: number) {
		let mx = this.map.get(x);
		if (!mx)
			return;//Not found

		let chunk = mx.get(y);
		if (!chunk)
			return;//Not found

		//Remove chunk
		chunk.destructor(this.state.renderer.gl);
		mx.delete(y);

		//console.log("Removed chunk at ", x, y);
	}

	getChunkBoundaries(): PreviewBoundary {
		return {
			start_x: Math.floor((this.boundary.center_x - this.boundary.width / 2.0) / CHUNK_SIZE),
			start_y: Math.floor((this.boundary.center_y - this.boundary.height / 2.0) / CHUNK_SIZE),
			end_x: Math.floor((this.boundary.center_x + this.boundary.width / 2.0) / CHUNK_SIZE) + 1,
			end_y: Math.floor((this.boundary.center_y + this.boundary.height / 2.0) / CHUNK_SIZE) + 1
		} as PreviewBoundary;
	}

	getPreviewBoundaries(zoom: number): PreviewBoundary {
		let div = CHUNK_SIZE * Math.pow(2, zoom);
		return {
			start_x: Math.floor((this.boundary.center_x - this.boundary.width / 2.0) / div),
			start_y: Math.floor((this.boundary.center_y - this.boundary.height / 2.0) / div),
			end_x: Math.floor((this.boundary.center_x + this.boundary.width / 2.0) / div) + 1,
			end_y: Math.floor((this.boundary.center_y + this.boundary.height / 2.0) / div) + 1
		} as PreviewBoundary;
	}

	drawChunks() {
		let boundary = this.getChunkBoundaries();
		let renderer = this.state.renderer;

		let needs_redraw = false;

		let cur_time_millis = (new Date()).getTime();

		this.iterChunksInBoundary(boundary, (chunk) => {
			chunk.processPixels(renderer.gl);
			if (chunk.render(renderer, cur_time_millis)) {
				needs_redraw = true;
			}
		});

		if (needs_redraw) {
			this.triggerRerender();
		}
	}

	drawPreviews() {
		let renderer = this.state.renderer;

		let needs_redraw = false;
		const cur_time_millis = (new Date()).getTime();

		//Reverse iterator
		for (let zoom = PREVIEW_SYSTEM_LAYER_COUNT; zoom >= 1; zoom--) {
			let layer = this.instance.preview_system.getLayer(zoom);
			if (!layer) continue;
			let boundary = this.getPreviewBoundaries(layer.zoom);
			const SIZE = CHUNK_SIZE * Math.pow(2, layer.zoom);

			layer.iterPreviewsInBoundary(boundary, (preview) => {
				if (preview.render(renderer, SIZE, cur_time_millis)) {
					needs_redraw = true;
				}
			});
		}

		if (needs_redraw) {
			this.triggerRerender();
		}
	}

	drawCursors() {
		let renderer = this.state.renderer;

		this.state.client.users.forEach((user: User) => {
			if (user == null)
				return;

			let zoom = this.scrolling.zoom;

			if (this.texture_cursor) {
				let width = this.texture_cursor.width / zoom;
				let height = this.texture_cursor.height / zoom;
				renderer.drawRect(
					renderer.shader_solid,
					this.texture_cursor, user.cursor_x + 0.5 - width / 2.0, user.cursor_y + 0.5 - height / 2.0,
					width, height);
			}
		})
	}

	drawCursorNicknames() {
		let renderer = this.state.renderer;

		this.state.client.users.forEach((user: User) => {
			if (user == null)
				return;

			let zoom = this.scrolling.zoom;

			let text = this.textCacheGet(renderer.gl, user.nickname);

			let width = text.width / zoom;
			let height = text.height / zoom;
			renderer.drawRect(renderer.shader_solid, text.tex, user.cursor_x - width / 2.0, user.cursor_y - height * 1.5, width, height);
		})
	}

	drawBrush() {
		if (!this.texture_brush) return;
		const cursor = this.instance.cursor;
		const renderer = this.state.renderer;
		let brush_size = cursor.tool_size;

		renderer.drawRect(
			renderer.shader_solid,
			this.texture_brush,
			cursor.canvas_x - brush_size / 2.0 + 0.5,
			cursor.canvas_y - brush_size / 2.0 + 0.5,
			brush_size, brush_size
		);
	}

	updateBoundary() {
		let boundary = this.boundary;
		let renderer = this.state.renderer;
		let canvas = renderer.params.canvas;
		boundary.center_x = -this.scrolling.x;
		boundary.center_y = -this.scrolling.y;
		boundary.width = canvas.width / this.scrolling.zoom;
		boundary.height = canvas.height / this.scrolling.zoom;
	}

	tick() {
	}

	draw() {
		if (!this.needs_redraw)
			return;

		this.needs_redraw = false;

		let renderer = this.state.renderer;

		renderer.viewportFullscreen();
		renderer.clear(1.0, 1.0, 1.0, 0.0);

		this.updateBoundary();

		let epsilon = 0.01;

		let boundary = this.boundary;
		renderer.setOrtho(
			boundary.center_x - boundary.width / 2.0 + epsilon,
			boundary.center_x + boundary.width / 2.0 + epsilon,
			boundary.center_y + boundary.height / 2.0 + epsilon,
			boundary.center_y - boundary.height / 2.0 + epsilon
		);

		this.drawPreviews();
		this.drawChunks();
		this.drawBrush();
		this.drawCursors();
		this.drawCursorNicknames();
	}

	getScrolling() {
		return this.scrolling;
	}

	addZoom(num: number) {
		let scrolling = this.getScrolling();

		let cursor = this.instance.cursor;

		let new_zoom = scrolling.zoom * (1.0 + num);
		new_zoom = Math.max(new_zoom, 0.0009765625); //0.5^10
		if (new_zoom > 80.0) new_zoom = 80.0;

		let canvas = this.state.renderer.params.canvas;

		let old_width = canvas.width / scrolling.zoom;
		let new_width = canvas.width / new_zoom;

		let old_height = canvas.height / scrolling.zoom;
		let new_height = canvas.height / new_zoom;

		let width_diff = new_width - old_width;
		let height_diff = new_height - old_height;

		scrolling.x += (cursor.x_norm - 0.5) * width_diff;
		scrolling.y += (cursor.y_norm - 0.5) * height_diff;

		this.setZoom(new_zoom)
	}

	setZoom(num: number) {
		this.scrolling.zoom = num;
		this.triggerRerender();
	}

	getZoom() {
		return this.scrolling.zoom;
	}

	putPixel(x: number, y: number, red: number, green: number, blue: number, alpha: number) {
		x = Math.floor(x);
		y = Math.floor(y);

		let localX = x % CHUNK_SIZE;
		let localY = y % CHUNK_SIZE;

		let chunkX = Math.floor(x / CHUNK_SIZE);
		let chunkY = Math.floor(y / CHUNK_SIZE);

		if (localX < 0) {
			localX += CHUNK_SIZE;
		}

		if (localY < 0) {
			localY += CHUNK_SIZE;
		}

		let chunk = this.state.map.getChunk(chunkX, chunkY);
		if (chunk) {
			chunk.putPixel(localX, localY, red, green, blue, alpha);
			this.triggerRerender();
		}
	}

	//returns array of 3 numbers (R,G,B)
	getPixel(x: number, y: number): number[] | undefined {
		x = Math.floor(x);
		y = Math.floor(y);

		let localX = x % CHUNK_SIZE;
		let localY = y % CHUNK_SIZE;

		let chunkX = Math.floor(x / CHUNK_SIZE);
		let chunkY = Math.floor(y / CHUNK_SIZE);

		if (localX < 0) {
			localX += CHUNK_SIZE;
		}

		if (localY < 0) {
			localY += CHUNK_SIZE;
		}

		let chunk = this.state.map.getChunk(chunkX, chunkY);
		if (chunk) {
			return chunk.getPixel(localX, localY);
		}

		return undefined;
	}
}

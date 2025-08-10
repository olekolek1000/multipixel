import { User } from "./client";
import { Texture } from "./render_engine";
import { ConnectedInstanceState, PREVIEW_SYSTEM_LAYER_COUNT, type RoomInstance } from "./room_instance";

export const CHUNK_SIZE = 256;

class PixelQueueCell {
	x: number;
	y: number;
	red: number;
	green: number;
	blue: number;

	constructor(x: number, y: number, red: number, green: number, blue: number) {
		this.x = x;
		this.y = y;
		this.red = red;
		this.green = green;
		this.blue = blue;
	}
}

class Chunk {
	x: number; // X position
	y: number; // Y position
	tex: Texture | null = null;
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

		this.pixels = new Uint8Array(CHUNK_SIZE * CHUNK_SIZE * 3);
		this.updateTexture(gl);

		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
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
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, CHUNK_SIZE, CHUNK_SIZE, 0, gl.RGB, gl.UNSIGNED_BYTE, this.pixels);
	}

	putPixel(x: number, y: number, red: number, green: number, blue: number) {
		this.pixel_queue.push(new PixelQueueCell(x, y, red, green, blue));
	}

	// returns array of 3 numbers (R,G,B)
	getPixel(x: number, y: number) {
		let data = this.pixels!;
		let image_offset = y * CHUNK_SIZE * 3 + x * 3;
		return [
			data[image_offset + 0],
			data[image_offset + 1],
			data[image_offset + 2]
		];
	}

	putImage(gl: WebGL2RenderingContext, dataview_rgb: DataView) {
		this.initTexture(gl);
		let offset = 0;
		let data = this.pixels!;

		for (let y = 0; y < CHUNK_SIZE; y++) {
			for (let x = 0; x < CHUNK_SIZE; x++) {
				let red = dataview_rgb.getUint8(offset + 0);
				let green = dataview_rgb.getUint8(offset + 1);
				let blue = dataview_rgb.getUint8(offset + 2);
				offset += 3;

				let image_offset = y * CHUNK_SIZE * 3 + x * 3;
				data[image_offset + 0] = red;
				data[image_offset + 1] = green;
				data[image_offset + 2] = blue;
			}
		}

		gl.bindTexture(gl.TEXTURE_2D, this.tex!.texture);
		this.updateTexture(gl);
	}

	processPixels(gl: WebGL2RenderingContext) {
		let count = this.pixel_queue.length;

		if (count == 0)
			return false;

		let data = this.pixels!;

		for (let i = 0; i < count; i++) {
			let cell = this.pixel_queue[i];

			let offset = cell.y * CHUNK_SIZE * 3 + cell.x * 3;
			data[offset + 0] = cell.red;
			data[offset + 1] = cell.green;
			data[offset + 2] = cell.blue;
		}

		this.updateTexture(gl);
		this.pixel_queue = [];

		return true;
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

		this.iterChunksInBoundary(boundary, (chunk) => {
			if (!chunk.tex) {
				return;
			}

			chunk.processPixels(renderer.gl);
			renderer.drawRect(
				renderer.params.color_keyed ? renderer.shader_color_keyed : renderer.shader_solid,
				chunk.tex,
				chunk.x * CHUNK_SIZE,
				chunk.y * CHUNK_SIZE,
				CHUNK_SIZE, CHUNK_SIZE);
		});
	}

	drawPreviews() {
		let renderer = this.state.renderer;

		//Reverse iterator
		let iter_count = 0;
		let render_count = 0;

		for (let zoom = PREVIEW_SYSTEM_LAYER_COUNT; zoom >= 1; zoom--) {
			let layer = this.instance.preview_system.getLayer(zoom);
			if (!layer) continue;
			let boundary = this.getPreviewBoundaries(layer.zoom);
			const SIZE = CHUNK_SIZE * Math.pow(2, layer.zoom);

			layer.iterPreviewsInBoundary(boundary, (preview) => {
				iter_count += 1;
				if (!preview.tex) {
					return;
				}

				render_count += 1;
				//console.log("preview zoom", zoom, "x", preview.x, "y", preview.y);
				renderer.drawRect(
					renderer.params.color_keyed ? renderer.shader_color_keyed : renderer.shader_solid,
					preview.tex,
					preview.x * SIZE,
					preview.y * SIZE,
					SIZE, SIZE);
			});
		}

		//console.log("iter count", iter_count, "render count", render_count);
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
		let cursor = this.instance.cursor;
		let renderer = this.state.renderer;
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
		renderer.clear(1.0, 1.0, 1.0, renderer.params.color_keyed ? 0.0 : 1.0);

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

	putPixel(x: number, y: number, red: number, green: number, blue: number) {
		x = Math.floor(x);
		y = Math.floor(y);

		let localX = x % CHUNK_SIZE;
		let localY = y % CHUNK_SIZE;

		let chunkX = Math.floor(x / CHUNK_SIZE);
		let chunkY = Math.floor(y / CHUNK_SIZE);

		if (localX < 0)
			localX += CHUNK_SIZE;

		if (localY < 0)
			localY += CHUNK_SIZE;

		let chunk = this.state.map.getChunk(chunkX, chunkY);
		if (chunk) {
			chunk.putPixel(localX, localY, red, green, blue);
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

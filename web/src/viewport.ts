import { User } from "./client";
import { Multipixel } from "./multipixel";
import { Texture } from "./render_engine";
import { lerp } from "./timestep";

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
	tex: Texture;
	pixels: Uint8Array | null;

	pixel_queue: Array<PixelQueueCell> = [];

	constructor(gl: WebGL2RenderingContext, x: number, y: number) {
		this.x = x;
		this.y = y;

		this.tex = new Texture();
		this.tex.texture = gl.createTexture()!;
		gl.bindTexture(gl.TEXTURE_2D, this.tex.texture);

		this.pixels = new Uint8Array(CHUNK_SIZE * CHUNK_SIZE * 3);
		this.updateTexture(gl);

		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
	}

	destructor(gl: WebGL2RenderingContext) {
		gl.deleteTexture(this.tex.texture);
		this.pixels = null;
	}

	updateTexture(gl: WebGL2RenderingContext) {
		gl.bindTexture(gl.TEXTURE_2D, this.tex.texture);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB, CHUNK_SIZE, CHUNK_SIZE, 0, gl.RGB, gl.UNSIGNED_BYTE, this.pixels);
		gl.generateMipmap(gl.TEXTURE_2D);
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

		gl.bindTexture(gl.TEXTURE_2D, this.tex.texture);
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

export class ChunkMap {
	multipixel: Multipixel;
	needs_redraw: boolean = true;
	texture_cursor!: Texture;
	texture_brush!: Texture;
	boundary_visual: Boundary = new Boundary();
	boundary_real: Boundary = new Boundary();
	text_cache = new Map<string, TextCacheCell>();
	map = new Map<number, Map<number, Chunk>>();

	scrolling = {
		x: 0,
		y: 0,
		zoom: 1.0,
		zoom_smooth: 1.0,
		zoom_smooth_prev: 1.0,
		zoom_interpolated: 1.0
	}

	constructor(multipixel: Multipixel) {
		this.multipixel = multipixel;
		let renderer = this.multipixel.getRenderer();

		renderer.loadTextureImage("public/img/cursor.png", (tex: Texture) => {
			this.texture_cursor = tex;
		});

		renderer.loadTextureImage("public/img/brush.png", (tex: Texture) => {
			this.texture_brush = tex;
		});

		window.addEventListener("resize", () => {
			this.resize();
		});

		this.resize();
		this.updateBoundaryReal();
		this.updateBoundaryVisual();
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
		let renderer = this.multipixel.getRenderer();
		let canvas = renderer.getCanvas();
		canvas.width = window.innerWidth;
		canvas.height = window.innerHeight;
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

		let new_chunk = new Chunk(this.multipixel.getRenderer().getContext(), x, y);
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
		chunk.destructor(this.multipixel.getRenderer().getContext());
		mx.delete(y);

		//console.log("Removed chunk at ", x, y);
	}

	getChunkBoundaries(boundary: Boundary) {
		return {
			start_x: Math.floor((boundary.center_x - boundary.width / 2.0) / CHUNK_SIZE),
			start_y: Math.floor((boundary.center_y - boundary.height / 2.0) / CHUNK_SIZE),
			end_x: Math.floor((boundary.center_x + boundary.width / 2.0) / CHUNK_SIZE) + 1,
			end_y: Math.floor((boundary.center_y + boundary.height / 2.0) / CHUNK_SIZE) + 1
		};
	}

	getChunkBoundariesVisual() {
		return this.getChunkBoundaries(this.boundary_visual);
	}

	getChunkBoundariesReal() {
		return this.getChunkBoundaries(this.boundary_real);
	}

	drawChunks(gl: WebGL2RenderingContext) {
		let boundary = this.getChunkBoundariesVisual();
		let renderer = this.multipixel.getRenderer();

		for (let y = boundary.start_y; y < boundary.end_y; y++) {
			for (let x = boundary.start_x; x < boundary.end_x; x++) {
				let chunk = this.getChunk(x, y);
				if (!chunk)
					continue;

				chunk.processPixels(gl);
				renderer.drawRect(
					chunk.tex,
					chunk.x * CHUNK_SIZE,
					chunk.y * CHUNK_SIZE,
					CHUNK_SIZE, CHUNK_SIZE);
			}
		}
	}

	drawCursors(gl: WebGL2RenderingContext) {
		let renderer = this.multipixel.getRenderer();

		this.multipixel.client.users.forEach((user: User) => {
			if (user == null)
				return;

			let zoom = this.scrolling.zoom_interpolated;

			if (this.texture_cursor) {
				let width = this.texture_cursor.width / zoom;
				let height = this.texture_cursor.height / zoom;
				renderer.drawRect(
					this.texture_cursor, user.cursor_x + 0.5 - width / 2.0, user.cursor_y + 0.5 - height / 2.0,
					width, height);
			}
		})
	}

	drawCursorNicknames(gl: WebGL2RenderingContext) {
		let renderer = this.multipixel.getRenderer();

		this.multipixel.client.users.forEach((user: User) => {
			if (user == null)
				return;

			let zoom = this.scrolling.zoom_interpolated;

			let text = this.textCacheGet(gl, user.nickname);

			let width = text.width / zoom;
			let height = text.height / zoom;
			renderer.drawRect(text.tex, user.cursor_x - width / 2.0, user.cursor_y - height * 1.5, width, height);
		})
	}

	drawBrush(gl: WebGL2RenderingContext) {
		if (!this.texture_brush) return;
		let cursor = this.multipixel.getCursor();
		let renderer = this.multipixel.getRenderer();
		let brush_size = cursor.brush_size;
		renderer.drawRect(
			this.texture_brush,
			cursor.canvas_x - brush_size / 2.0 + 0.5,
			cursor.canvas_y - brush_size / 2.0 + 0.5,
			brush_size, brush_size
		);
	}

	updateBoundaryVisual() {
		let boundary = this.boundary_visual;
		let renderer = this.multipixel.getRenderer();
		let canvas = renderer.getCanvas();
		boundary.center_x = -this.scrolling.x;
		boundary.center_y = -this.scrolling.y;
		boundary.width = canvas.width / this.scrolling.zoom_interpolated;
		boundary.height = canvas.height / this.scrolling.zoom_interpolated;
	}

	updateBoundaryReal() {
		let boundary = this.boundary_real;
		let renderer = this.multipixel.getRenderer();
		let canvas = renderer.getCanvas();
		boundary.center_x = -this.scrolling.x;
		boundary.center_y = -this.scrolling.y;
		boundary.width = canvas.width / this.scrolling.zoom;
		boundary.height = canvas.height / this.scrolling.zoom;
	}

	tick() {
		this.scrolling.zoom_smooth_prev = this.scrolling.zoom_smooth;
		this.scrolling.zoom_smooth = lerp(0.2, this.scrolling.zoom_smooth, this.scrolling.zoom);
	}

	draw() {
		if (!this.needs_redraw)
			return;

		this.needs_redraw = false;

		let renderer = this.multipixel.getRenderer();
		let gl = renderer.getContext();

		renderer.viewportFullscreen();
		renderer.clear(0.8, 0.8, 0.8, 1);

		let alpha = this.multipixel.timestep.getAlpha();

		this.scrolling.zoom_interpolated = lerp(alpha, this.scrolling.zoom_smooth_prev, this.scrolling.zoom_smooth);

		this.updateBoundaryVisual();
		this.updateBoundaryReal();

		let epsilon = 0.01;

		let boundary = this.boundary_visual;
		renderer.setOrtho(
			boundary.center_x - boundary.width / 2.0 + epsilon,
			boundary.center_x + boundary.width / 2.0 + epsilon,
			boundary.center_y + boundary.height / 2.0 + epsilon,
			boundary.center_y - boundary.height / 2.0 + epsilon
		);

		this.drawChunks(gl);
		this.drawBrush(gl);
		this.drawCursors(gl);
		this.drawCursorNicknames(gl);

		if (Math.abs(this.scrolling.zoom_smooth - this.scrolling.zoom) > 0.001)
			this.triggerRerender();
	}

	getScrolling() {
		return this.scrolling;
	}

	addZoom(num: number) {
		let scrolling = this.getScrolling();
		let zoom = scrolling.zoom * (1.0 + num);
		this.setZoom(zoom)
	}

	setZoom(num: number) {
		if (num < 0.1) num = 0.1;
		if (num > 80.0) num = 80.0;
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

		let chunk = this.multipixel.map.getChunk(chunkX, chunkY);
		if (chunk) {
			chunk.putPixel(localX, localY, red, green, blue);
			this.triggerRerender();
		}
	}

	//returns array of 3 numbers (R,G,B)
	getPixel(x: number, y: number) {
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

		let chunk = this.multipixel.map.getChunk(chunkX, chunkY);
		if (chunk)
			return chunk.getPixel(localX, localY);

		return null;
	}
}

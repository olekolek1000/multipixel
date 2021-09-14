const chunk_size = 256;

class PixelQueueCell {
	x;
	y;
	red;
	green;
	blue;
	constructor(x, y, red, green, blue) {
		this.x = x;
		this.y = y;
		this.red = red;
		this.green = green;
		this.blue = blue;
	}
}

function getViewport() {
	return document.getElementById("mp_viewport");
}

function getBody() {
	return document.getElementById("body");
}

class Chunk {
	canvas;
	ctx;
	x;//X position
	y;//y position
	image_data;

	pixel_queue = [];

	constructor(x, y) {
		this.x = x;
		this.y = y;
		this.canvas = document.createElement("canvas");
		this.canvas.width = chunk_size;
		this.canvas.height = chunk_size;
		this.ctx = this.canvas.getContext("2d");
		this.ctx.imageSmoothingEnabled = false;

		//this.ctx.fillStyle = '#555';
		//this.ctx.fillRect(0, 0, chunk_size, chunk_size);
		this.image_data = this.ctx.getImageData(0, 0, chunk_size, chunk_size);
	}

	destructor = function () {
		this.canvas.remove();
		this.ctx = null;
		this.ctx = null;
	}

	putPixel = function (x, y, red, green, blue) {
		this.pixel_queue.push(new PixelQueueCell(x, y, red, green, blue));
	}

	// returns array of 3 numbers (R,G,B)
	getPixel = function (x, y) {
		let data = this.image_data.data;
		let image_offset = y * chunk_size * 4 + x * 4;
		return [
			data[image_offset + 0],
			data[image_offset + 1],
			data[image_offset + 2]
		];
	}

	putImage = function (dataview_rgb) {
		let offset = 0;

		let data = this.image_data.data;

		for (let y = 0; y < 256; y++) {
			for (let x = 0; x < 256; x++) {
				let red = dataview_rgb.getUint8(offset + 0);
				let green = dataview_rgb.getUint8(offset + 1);
				let blue = dataview_rgb.getUint8(offset + 2);
				offset += 3;

				let image_offset = y * chunk_size * 4 + x * 4;
				data[image_offset + 0] = red;
				data[image_offset + 1] = green;
				data[image_offset + 2] = blue;
				data[image_offset + 3] = 255;
			}
		}

		this.ctx.putImageData(this.image_data, 0, 0);
	}

	processPixels = function () {
		let count = this.pixel_queue.length;

		if (count == 0)
			return false;

		let data = this.image_data.data;

		for (let i = 0; i < count; i++) {
			let cell = this.pixel_queue[i];

			let offset = cell.y * chunk_size * 4 + cell.x * 4;
			data[offset + 0] = cell.red;
			data[offset + 1] = cell.green;
			data[offset + 2] = cell.blue;
		}

		this.ctx.putImageData(this.image_data, 0, 0);

		this.pixel_queue = [];

		return true;
	}
}

//Load cursor
var cursor_img = document.createElement("img");
cursor_img.src = "breeze.png";

class Map {
	viewport;
	viewport_ctx;
	needs_redraw;

	map = [];

	scrolling = {
		x: 0,
		y: 0,
		zoom: 1.0
	};

	constructor() {
		this.viewport = getViewport();
		this.viewport_ctx = this.viewport.getContext("2d");
		this.needs_redraw = true;

		window.addEventListener("resize", () => { this.resize(); });
		this.resize();
	}

	triggerRerender = function () {
		this.needs_redraw = true;
	}

	resize = function () {
		let viewport = getViewport();
		viewport.width = window.innerWidth;
		viewport.height = window.innerHeight;
		this.triggerRerender();
	}

	chunkExists = function (x, y) {
		if (this.map[x] == null)
			return false;

		if (this.map[x][y] == null)
			return false;

		return true;
	}

	getChunk = function (x, y) {
		if (!this.chunkExists(x, y))
			return null;

		return this.map[x][y];
	}

	createChunk = function (x, y) {
		if (this.map[x] == null) {
			this.map[x] = [];
		}

		if (this.map[x][y] != null) {
			//Chunk already created, return existing
			return this.map[x][y];
		}

		let ch = new Chunk(x, y);
		this.map[x][y] = ch;

		//console.log("Created chunk at ", x, y);

		return ch;
	}

	removeChunk = function (x, y) {
		if (this.map[x] == null)
			return;//not found

		if (this.map[x][y] != null) {
			let chunk = this.map[x][y];
			chunk.destructor();
			this.map[x][y] = null;
		}

		//console.log("Removed chunk at ", x, y);
	}

	getRenderX = function (pos_x) {
		return pos_x * this.scrolling.zoom + this.scrolling.x;
	}

	getRenderY = function (pos_y) {
		return pos_y * this.scrolling.zoom + this.scrolling.y;
	}

	getBoundaries = function () {
		return {
			start_x: Math.floor((-this.scrolling.x / this.scrolling.zoom) / chunk_size),
			start_y: Math.floor((-this.scrolling.y / this.scrolling.zoom) / chunk_size),
			end_x: Math.floor(((-this.scrolling.x + this.viewport.width) / this.scrolling.zoom) / chunk_size) + 1,
			end_y: Math.floor(((-this.scrolling.y + this.viewport.height) / this.scrolling.zoom) / chunk_size) + 1
		};
	}

	drawChunks = function () {
		this.viewport_ctx.fillStyle = '#ccc';
		this.viewport_ctx.fillRect(0, 0, this.viewport.width, this.viewport.height);

		let boundary = this.getBoundaries();

		this.viewport_ctx.imageSmoothingEnabled = this.scrolling.zoom < 1.0;

		for (let y = boundary.start_y; y < boundary.end_y; y++) {
			for (let x = boundary.start_x; x < boundary.end_x; x++) {
				let chunk = this.getChunk(x, y);
				if (!chunk)
					continue;

				chunk.processPixels();

				let render_x = this.getRenderX(chunk.x * chunk_size);
				let render_y = this.getRenderY(chunk.y * chunk_size);
				let render_size = chunk_size * this.scrolling.zoom;

				if (this.scrolling.zoom < 1.0)
					render_size++;//Fix for Firefox (prevent pixel-wide lines in chunk borders)

				this.viewport_ctx.drawImage(chunk.canvas, render_x, render_y, render_size, render_size);
			}
		}
	}

	drawCursors = function () {
		this.viewport_ctx.font = "10px Helvetica";
		this.viewport_ctx.fillStyle = '#000';

		client.users.forEach(user => {
			if (user == null)
				return;

			let render_x = this.getRenderX(user.cursor_x + 0.5);
			let render_y = this.getRenderY(user.cursor_y + 0.5);

			this.viewport_ctx.drawImage(cursor_img, render_x, render_y);

			this.viewport_ctx.fillText(user.nickname, render_x + cursor_img.width + 4, render_y + cursor_img.height - 10);
		})
	}

	draw = function () {
		if (!this.needs_redraw)
			return;
		this.needs_redraw = false;
		this.drawChunks();
		this.drawCursors();
	}

	getScrolling = function () {
		return this.scrolling;
	}

	addZoom = function (num) {
		let scrolling = this.getScrolling();
		let zoom = scrolling.zoom * (1.0 + num);
		this.setZoom(zoom)
	}

	setZoom = function (num) {
		let scrolling = this.getScrolling();
		let zoom = num;
		let zoom_prev = scrolling.zoom;
		zoom = clamp(zoom, 0.1, 64.0);

		if (zoom == zoom_prev)
			return;

		let zoom_mul = zoom / zoom_prev;

		scrolling.zoom = zoom;
		scrolling.x *= zoom_mul;
		scrolling.y *= zoom_mul;

		let pixel_diff_x = this.viewport.width / zoom_prev - this.viewport.width / zoom;
		let pixel_diff_y = this.viewport.height / zoom_prev - this.viewport.height / zoom;

		scrolling.x -= (pixel_diff_x * zoom) * (mouse.x / this.viewport.width);
		scrolling.y -= (pixel_diff_y * zoom) * (mouse.y / this.viewport.height);

		this.triggerRerender();
	}

	putPixel = function (x, y, red, green, blue) {
		x = Math.floor(x);
		y = Math.floor(y);

		let localX = x % chunk_size;
		let localY = y % chunk_size;

		let chunkX = Math.floor(x / chunk_size);
		let chunkY = Math.floor(y / chunk_size);

		if (localX < 0)
			localX += chunk_size;

		if (localY < 0)
			localY += chunk_size;

		let chunk = map.getChunk(chunkX, chunkY);
		if (chunk) {
			chunk.putPixel(localX, localY, red, green, blue);
			this.triggerRerender();
		}
	}

	//returns array of 3 numbers (R,G,B)
	getPixel = function (x, y) {
		x = Math.floor(x);
		y = Math.floor(y);

		let localX = x % chunk_size;
		let localY = y % chunk_size;

		let chunkX = Math.floor(x / chunk_size);
		let chunkY = Math.floor(y / chunk_size);

		if (localX < 0)
			localX += chunk_size;

		if (localY < 0)
			localY += chunk_size;

		let chunk = map.getChunk(chunkX, chunkY);
		if (chunk)
			return chunk.getPixel(localX, localY);

		return null;
	}
}

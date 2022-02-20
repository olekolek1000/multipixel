import { ChunkMap, CHUNK_SIZE } from "./chunk_map";
import { Client, User } from "./client";
import { Chat } from "./chat";
import { RenderEngine } from "./render_engine";
import { lerp, Timestep } from "./timestep";
import { ColorPalette } from "./color";
import { Preview, PreviewLayer, PreviewSystem } from "./preview_system";

function clamp(num: number, min: number, max: number) {
	return num <= min ? min : num >= max ? max : num;
}

function dec2hex(n: number) {
	n = Math.round(n);
	if (n < 0) n = 0;
	if (n > 255) n = 255;
	return n.toString(16).padStart(2, '0');
}

export function rgb2hex(red: number, green: number, blue: number) {
	return '#' + dec2hex(red) + dec2hex(green) + dec2hex(blue);
}

enum ToolID {
	Brush = 0,
	Floodfill = 1
}

export class Cursor {
	just_pressed_down: boolean = false;
	x: number = 0.0;
	y: number = 0.0;
	x_prev: number = 0.0;
	y_prev: number = 0.0;
	canvas_x: number = 0.0;
	canvas_x_smooth: number = 0.0;
	canvas_y: number = 0.0;
	canvas_y_smooth: number = 0.0;
	down_left: boolean = false;
	down_right: boolean = false;
	brush_size: number = 1;
	tool_id: number = ToolID.Brush;
}

export class Multipixel {
	client: Client;
	map!: ChunkMap;
	preview_system!: PreviewSystem;
	chat!: Chat;
	renderer!: RenderEngine;
	cursor!: Cursor;
	timestep!: Timestep;
	palette!: ColorPalette;
	events_enabled: boolean = true;
	needs_boundaries_update: boolean = true;

	constructor(host: string, nickname: string, room_name: string, done_callback: () => void) {
		document.title = "#" + room_name + " - MultiPixel";
		this.client = new Client(this, host, nickname, room_name, () => {
			this.onConnect(done_callback);
		});
	}

	onConnect(done_callback: () => void) {
		//Init renderer
		this.renderer = new RenderEngine(document.getElementById("canvas_render") as HTMLCanvasElement);

		//Init chunk map
		this.map = new ChunkMap(this);

		//Init preview system
		this.preview_system = new PreviewSystem(this);

		//Init cursor
		this.cursor = new Cursor();

		//Init chat
		this.chat = new Chat(this.client);

		this.initGUI();
		this.initListeners();
		this.initTimestep();

		//Start rendering
		this.draw();

		done_callback();
	}

	draw() {
		let i = 0;
		while (this.timestep.onTick() && i < 1000) {
			this.tick();
			i++;
		}
		this.map.draw();
		window.requestAnimationFrame(() => {
			this.draw();
		});
	}

	initGUI() {
		let slider = document.getElementById("mp_slider_brush_size") as HTMLInputElement;
		slider.value = "1";
		slider.addEventListener("change", () => {
			let size = slider.value;
			let num_size = Number.parseFloat(size);
			this.getCursor().brush_size = num_size;
			this.client.socketSendBrushSize(num_size);
		});

		(document.getElementById("mp_slider_brush_smoothing") as HTMLInputElement).value = "0.0";

		let button_zoom_1_1 = document.getElementById("button_zoom_1_1") as HTMLElement;
		button_zoom_1_1.addEventListener("click", () => {
			this.map.setZoom(1.0);
			this.map.triggerRerender();
		});

		let button_undo = document.getElementById("button_undo") as HTMLElement;
		button_undo.addEventListener("click", () => {
			this.client.socketSendUndo();
		});

		document.addEventListener('keydown', (event) => {
			if (event.ctrlKey && event.key === 'z') {
				this.client.socketSendUndo();
			}
		});

		let button_tool_brush = document.getElementById("button_tool_brush") as HTMLElement;
		button_tool_brush.addEventListener("click", () => {
			this.selectTool(ToolID.Brush);
			this.markSelectedTool(button_tool_brush);
		});

		let button_tool_floodfill = document.getElementById("button_tool_floodfill") as HTMLElement;
		button_tool_floodfill.addEventListener("click", () => {
			this.selectTool(ToolID.Floodfill);
			this.markSelectedTool(button_tool_floodfill);
		});

		this.palette = new ColorPalette(this, document.getElementById("mpc_color_palette") as HTMLElement);
	}

	setEventsEnabled(enabled: boolean) {
		this.events_enabled = enabled;
	}

	initListeners() {
		setInterval(() => { this.updateBoundary() }, 200);

		setInterval(() => { this.client.socketSendPing() }, 8000);

		let canvas = this.renderer.getCanvas();
		let body = document.getElementById("body") as HTMLElement;

		canvas.addEventListener("mousemove", (e: MouseEvent) => {
			let cursor = this.getCursor();
			cursor.x_prev = cursor.x;
			cursor.y_prev = cursor.y;
			cursor.x = e.clientX;
			cursor.y = e.clientY;

			let canvas = this.renderer.getCanvas();

			let boundary = this.map.boundary_visual;
			let scrolling = this.map.scrolling;

			let raw_x = boundary.center_x - boundary.width / 2.0 + (cursor.x / canvas.width) * boundary.width;
			let raw_y = boundary.center_y - boundary.height / 2.0 + (cursor.y / canvas.height) * boundary.height;

			cursor.canvas_x = Math.floor(raw_x);
			cursor.canvas_y = Math.floor(raw_y);

			let smooth = false;
			let smooth_val = 1.0;

			if (this.cursor.down_left && this.cursor.tool_id == ToolID.Brush) {
				let value = parseInt((document.getElementById("mp_slider_brush_smoothing") as HTMLInputElement).value);
				if (value > 0) {
					smooth = true;
					smooth_val = 1.0 - value / 101.0;
				}
			}

			if (smooth) {
				cursor.canvas_x_smooth = lerp(smooth_val, cursor.canvas_x_smooth, cursor.canvas_x);
				cursor.canvas_y_smooth = lerp(smooth_val, cursor.canvas_y_smooth, cursor.canvas_y);

				if (cursor.just_pressed_down) {
					cursor.canvas_x_smooth = cursor.canvas_x;
					cursor.canvas_y_smooth = cursor.canvas_y;
				}
			}

			this.client.socketSendCursorPos(smooth ? cursor.canvas_x_smooth : cursor.canvas_x, smooth ? cursor.canvas_y_smooth : cursor.canvas_y);

			if (cursor.down_right) {
				//Scroll
				scrolling.x += (cursor.x - cursor.x_prev) / scrolling.zoom;
				scrolling.y += (cursor.y - cursor.y_prev) / scrolling.zoom;
				this.needs_boundaries_update = true;
			}

			this.map.triggerRerender();
			cursor.just_pressed_down = false;
		});

		canvas.addEventListener("mousedown", (e: MouseEvent) => {
			if (!this.events_enabled) return;
			let cursor = this.getCursor();
			cursor.just_pressed_down = true;

			if (e.button == 0) { // Left
				if (this.map.getZoom() < 0.5) {
					this.map.setZoom(0.5);
					this.needs_boundaries_update = true;
				}
				else {
					cursor.down_left = true;
					this.client.socketSendCursorDown();
				}
			}

			if (e.button == 1) {
				this.performColorPick();
			}

			if (e.button == 2) { // Right
				cursor.down_right = true;
			}
		});


		canvas.addEventListener("mouseup", (e: MouseEvent) => {
			if (!this.events_enabled) return;
			let cursor = this.getCursor();

			if (e.button == 0) { // Left
				cursor.down_left = false;
				this.client.socketSendCursorUp();
			}

			if (e.button == 2) { // Right
				cursor.down_right = false;
			}
		});

		window.addEventListener("blur", (e) => {
			let cursor = this.getCursor();
			e;
			cursor.down_left = false;
			this.client.socketSendCursorUp();
		});

		canvas.addEventListener("wheel", (e: WheelEvent) => {
			if (!this.events_enabled) return;
			let zoom_diff = clamp(-e.deltaY * 100.0, -1, 1) * 0.2;
			this.map.addZoom(zoom_diff);
			this.needs_boundaries_update = true;
		});

		body.addEventListener("contextmenu", (e) => {
			e.preventDefault();
			return false;
		});
	}

	initTimestep() {
		this.timestep = new Timestep();
		this.timestep.setRate(60.0);
	}

	getRenderer() {
		return this.renderer;
	}

	getCursor() {
		return this.cursor;
	}

	refreshPlayerList() {
		let player_list = document.getElementById("mp_player_list") as HTMLElement;
		let buf = "[Online players]<br><br>";

		let self_shown = false;
		let t = this;
		function add_self() {
			self_shown = true;
			buf += "You [" + t.client.id + "]<br>";
		}

		this.client.users.forEach((user: User) => {
			if (user == null)
				return;

			if (!self_shown && this.client.id < user.id) {
				add_self();
				self_shown = true;
			}

			buf += user.nickname + " [" + user.id + "]<br>";
		})

		if (!self_shown)
			add_self();

		player_list.innerHTML = buf;
	}

	selectTool(tool_id: ToolID) {
		this.cursor.tool_id = tool_id;
		this.client.socketSendToolType(tool_id);
	}

	markSelectedTool(selected_element: HTMLElement) {
		let elements = document.getElementsByClassName("button_tool");
		for (const element of elements) {
			element.classList.remove("button_tool_selected");
		}

		selected_element.classList.add("button_tool_selected");
	}

	updateBoundary() {
		if (!this.needs_boundaries_update)
			return;
		this.needs_boundaries_update = false;
		this.client.socketSendBoundary();

		let camera_zoom = this.map.getZoom();

		//Calculate preview visibility
		let target_zoom: number = 0;
		if (camera_zoom < 0.0625) {
			target_zoom = 4;
		}
		else if (camera_zoom < 0.125) {
			target_zoom = 3;
		}
		else if (camera_zoom < 0.25) {
			target_zoom = 2;
		} else if (camera_zoom < 0.5) {
			target_zoom = 1;
		}

		console.log("target zoom " + target_zoom);

		for (let zoom = 1; zoom <= 4; zoom++) {
			let layer = this.preview_system.getOrCreateLayer(zoom);
			let boundary = this.map.getPreviewBoundariesVisual(layer.zoom);

			const SIZE = CHUNK_SIZE * Math.pow(2, layer.zoom);

			for (let y = boundary.start_y; y < boundary.end_y; y++) {
				for (let x = boundary.start_x; x < boundary.end_x; x++) {
					let load_mode = layer.zoom == target_zoom;
					let preview: Preview;
					if (load_mode) {
						preview = layer.getOrCreatePreview(x, y);
						this.client.socketSendPreviewRequest(x, y, layer.zoom);
					}
					else {
						let p = layer.getPreview(x, y);
						if (p) {
							preview = p;
							preview.remove_timeout++;

							if (preview.remove_timeout > 10) {
								layer.removePreview(x, y);
								continue;
							}
						}
						else continue;//Not loaded
					}
				}
			}
		}
	}

	performColorPick() {
		let cursor = this.getCursor();
		let rgb = this.map.getPixel(cursor.canvas_x, cursor.canvas_y);
		if (rgb) {
			this.palette.setColor({ r: rgb[0], g: rgb[1], b: rgb[2] });
		}
	}

	tick() {
		this.map.tick();
	}
}

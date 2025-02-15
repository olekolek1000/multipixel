import { ChunkMap, CHUNK_SIZE } from "./chunk_map";
import { Client, User } from "./client";
import { Chat } from "./chat";
import { RenderEngine } from "./render_engine";
import { lerp, Timestep } from "./timestep";
import { Preview, PreviewLayer, PreviewSystem } from "./preview_system";
import { globals } from ".";
import { RoomRefs, RoomScreen, RoomScreenGlobals } from "./room_screen"
import React from "react";
import { ToolboxGlobals } from "./tool_panel";
import style from "./style.scss"
import tool from "./tool"

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

export class Cursor {
	just_pressed_down: boolean = false;
	x_norm: number = 0.0;
	y_norm: number = 0.0;
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
	tool_size: number = 1;
	tool_id: tool.ToolID = tool.ToolID.Brush;
}

export const LAYER_COUNT = 5;

export class Multipixel {
	client: Client;
	map!: ChunkMap;
	preview_system!: PreviewSystem;
	chat!: Chat;
	renderer!: RenderEngine;
	cursor!: Cursor;
	timestep!: Timestep;
	events_enabled: boolean = true;
	needs_boundaries_update: boolean = true;
	room_refs: RoomRefs | null = null;
	toolbox_globals!: ToolboxGlobals;
	room_screen_globals!: RoomScreenGlobals;

	callback_player_update: (() => void) | null = null;

	constructor(params: {
		host: string;
		nickname: string;
		room_name: string;
		connection_callback: (error_str?: string) => void;
	}) {
		document.title = "#" + params.room_name + " - MultiPixel";
		this.toolbox_globals = new ToolboxGlobals(this);
		this.room_screen_globals = new RoomScreenGlobals();
		this.client = new Client({
			multipixel: this,
			address: params.host,
			nickname: params.nickname,
			room_name: params.room_name,
			connection_callback: (error_str) => {
				if (error_str) {
					params.connection_callback(error_str);
					return;
				}

				this.initialize();
				params.connection_callback(undefined);
			}
		})
	}

	initialize() {
		globals.setState(<RoomScreen globals={this.room_screen_globals} multipixel={this} refs_callback={(refs) => {
			this.room_refs = refs;

			//Init renderer
			this.renderer = new RenderEngine(refs.canvas_render as HTMLCanvasElement);

			//Init chunk map
			this.map = new ChunkMap(this);

			//Init preview system
			this.preview_system = new PreviewSystem(this);

			//Init cursor
			this.cursor = new Cursor();

			//Init chat
			this.chat = new Chat(this.client);

			this.initGUI(refs);
			this.initListeners(refs);
			this.initTimestep();

			//Start rendering
			this.draw();

			//Init client protocol
			this.client.initProtocol();
		}} />);
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

	handleButtomZoom1_1() {
		this.map.setZoom(1.0);
		this.map.triggerRerender();
	}

	handleButtonUndo() {
		this.client.socketSendUndo();
	}

	initGUI(refs: RoomRefs) {
		document.addEventListener('keydown', (event) => {
			if (event.ctrlKey && event.key === 'z') {
				this.client.socketSendUndo();
			}
		});
	}

	setEventsEnabled(enabled: boolean) {
		this.events_enabled = enabled;
	}

	initListeners(refs: RoomRefs) {
		setInterval(() => { this.updateBoundary() }, 200);

		setInterval(() => { this.client.socketSendPing() }, 8000);

		let canvas = this.renderer.getCanvas();

		canvas.addEventListener("mousemove", (e: MouseEvent) => {
			let cursor = this.getCursor();
			cursor.x_prev = cursor.x;
			cursor.y_prev = cursor.y;
			cursor.x = e.clientX * this.renderer.display_scale;
			cursor.y = e.clientY * this.renderer.display_scale;
			cursor.x_norm = e.clientX / window.innerWidth;
			cursor.y_norm = e.clientY / window.innerHeight;

			let canvas = this.renderer.getCanvas();

			let boundary = this.map.boundary;
			let scrolling = this.map.scrolling;

			let raw_x = boundary.center_x - boundary.width / 2.0 + (cursor.x / canvas.width) * boundary.width;
			let raw_y = boundary.center_y - boundary.height / 2.0 + (cursor.y / canvas.height) * boundary.height;

			cursor.canvas_x = Math.floor(raw_x);
			cursor.canvas_y = Math.floor(raw_y);

			let smooth = false;
			let smooth_val = 1.0;

			if (this.cursor.down_left && tool.supportsSmoothing(this.cursor.tool_id)) {
				//Check brush smoothing
				if (this.toolbox_globals.param_tool_smoothing) {
					smooth = true;
					smooth_val = 1.0 - Math.pow(this.toolbox_globals.param_tool_smoothing, 0.1) / 1.01;
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

			this.room_screen_globals.setMousePosText(<span className={style.cursor_pos}>{"X " + cursor.canvas_x}<br />{"Y " + cursor.canvas_y}</span>);

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
				if (this.map.getZoom() < 1.0) {
					this.map.setZoom(1.0);
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
			let zoom_diff = -e.deltaY * 0.0015;
			this.map.addZoom(zoom_diff);
			this.needs_boundaries_update = true;
		});

		globals.root.addEventListener("contextmenu", (e) => {
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

	updatePlayerList() {
		if (this.callback_player_update)
			this.callback_player_update();
	}

	getPlayerList(): Array<string> {
		let arr = new Array<string>();

		arr.push("You");

		this.client.users.forEach((user: User) => {
			if (!user) return;
			arr.push(user.nickname);
		});

		return arr;
	}

	selectTool(tool_id: tool.ToolID) {
		this.cursor.tool_id = tool_id;
		this.client.socketSendToolType(tool_id);
	}

	updateBoundary() {
		if (!this.needs_boundaries_update)
			return;
		this.needs_boundaries_update = false;
		this.client.socketSendBoundary();

		let camera_zoom = this.map.getZoom();

		//Calculate preview visibility
		let target_zoom: number = 0;
		if (camera_zoom < Math.pow(0.5, 5)) {
			target_zoom = 5;
		} else if (camera_zoom < Math.pow(0.5, 4)) {
			target_zoom = 4;
		} else if (camera_zoom < Math.pow(0.5, 3)) {
			target_zoom = 3;
		} else if (camera_zoom < Math.pow(0.5, 2)) {
			target_zoom = 2;
		} else if (camera_zoom < Math.pow(0.5, 1)) {
			target_zoom = 1;
		}

		//console.log("target zoom " + target_zoom);
		let request_sent_count = 0;

		for (let zoom = 1; zoom <= LAYER_COUNT; zoom++) {
			let layer = this.preview_system.getOrCreateLayer(zoom);
			let boundary = this.map.getPreviewBoundaries(layer.zoom);

			const SIZE = CHUNK_SIZE * Math.pow(2, layer.zoom);

			for (let y = boundary.start_y; y < boundary.end_y; y++) {
				for (let x = boundary.start_x; x < boundary.end_x; x++) {
					let load_mode = layer.zoom == target_zoom;
					let preview: Preview | null;
					if (load_mode && request_sent_count < 30) {
						preview = layer.getPreview(x, y);
						if (!preview) {
							preview = layer.getOrCreatePreview(x, y);
							this.client.socketSendPreviewRequest(x, y, layer.zoom);
							request_sent_count++;
							//console.log("sent request");
						}
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

		if (request_sent_count > 0) {
			this.needs_boundaries_update = true;
		}
	}

	performColorPick() {
		let cursor = this.getCursor();
		let rgb = this.map.getPixel(cursor.canvas_x, cursor.canvas_y);
		if (rgb) {
			let cp = this.toolbox_globals.color_palette;
			if (cp) {
				cp.setColor({ r: rgb[0], g: rgb[1], b: rgb[2] });
			}
		}
	}

	tick() {
		this.map.tick();
	}

	setProcessingStatusText(text: string) {
		this.room_screen_globals.setProcessingStatusText(text);
	}
}

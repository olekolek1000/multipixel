
import { Chat } from "./chat/chat";
import { ChunkMap } from "./chunk_map";
import { Client, User } from "./client";
import { PreviewSystem } from "./preview_system";
import { RenderEngine } from "./render_engine";
import { lerp } from "./timestep";
import tool from "./tool";
import type { ToolboxGlobals } from "./tool_panel";
import { RoomScreen, type RoomScreenGlobals, type RoomScreenRefs } from "./views/canvas/room_screen";
import style from "./style.module.scss"
import type { ConnectParams } from "./multipixel";
import { globals } from ".";

export const PREVIEW_SYSTEM_LAYER_COUNT = 10;

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

export class ConnectedInstanceState {
	client: Client;
	chat: Chat;
	canvas_render: HTMLCanvasElement;
	renderer: RenderEngine;
	map: ChunkMap;

	constructor(params: {
		instance: RoomInstance,
		client: Client;
		canvas_render: HTMLCanvasElement;
		renderer: RenderEngine;
	}) {
		this.renderer = params.renderer;
		this.client = params.client;
		this.chat = new Chat(params.client);
		this.map = new ChunkMap(params.instance, this);
		this.canvas_render = params.canvas_render;
	}
}


export class RoomInstance {
	state?: ConnectedInstanceState;
	toolbox_globals: ToolboxGlobals;
	room_screen_globals: RoomScreenGlobals;
	preview_system: PreviewSystem;
	cursor: Cursor;

	private needs_boundaries_update: boolean;

	callback_user_update: (() => void) | null = null;

	constructor(params: {
		connect_params: ConnectParams,
		toolbox_globals: ToolboxGlobals,
		room_screen_globals: RoomScreenGlobals,
		connection_callback: (error_str?: string) => void;
	}) {
		document.title = "#" + params.connect_params.room_name + " - MultiPixel";

		const client = new Client({
			instance: this,
			address: params.connect_params.host,
			nickname: params.connect_params.nickname,
			room_name: params.connect_params.room_name,
			connection_callback: (error_str) => {
				if (error_str) {
					params.connection_callback(error_str);
					return;
				}

				params.connection_callback(undefined);
				this.initRoomScreen(client);
			}
		})

		this.toolbox_globals = params.toolbox_globals;
		this.room_screen_globals = params.room_screen_globals;

		this.preview_system = new PreviewSystem(this);
		this.cursor = new Cursor();

		this.needs_boundaries_update = true;
	}

	private initRoomScreen(client: Client) {
		globals.setState(<RoomScreen globals={this.room_screen_globals} instance={this} refs_callback={(refs) => {
			const state = new ConnectedInstanceState({
				instance: this,
				client,
				canvas_render: refs.canvas_render,
				renderer: new RenderEngine({
					canvas: refs.canvas_render,
				}),
			});
			this.state = state;
			this.initEvents(refs);
			state.client.initProtocol();
		}} />);
	}

	private initEvents(refs: RoomScreenRefs) {
		const el = refs.canvas_render;
		el.addEventListener("mousemove", (e: MouseEvent) => {
			this.handleCursorMoveEvent(e);
		});

		el.addEventListener("mousedown", (e: MouseEvent) => {
			this.handleCursorDownEvent(e);
		});

		el.addEventListener("mouseup", (e: MouseEvent) => {
			this.handleCursorUpEvent(e);
		});

		el.addEventListener("wheel", (e: WheelEvent) => {
			this.handleWheelEvent(e);
		});
	}

	tick() {

	}

	draw() {
		if (!this.state) return;
		this.state.map.draw();
	}

	actionZoom1_1() {
		if (!this.state) return;
		this.state.map.setZoom(1.0);
		this.state.map.triggerRerender();
	}

	actionUndo() {
		if (this.state) {
			this.state.client.socketSendUndo();
		}
	}

	private actionColorPick() {
		if (!this.state) return;
		let rgb = this.state.map.getPixel(this.cursor.canvas_x, this.cursor.canvas_y);
		if (rgb === undefined) {
			return;
		}
		let cp = this.toolbox_globals.color_palette;
		if (cp) {
			cp.setColor({ r: rgb[0], g: rgb[1], b: rgb[2] });
		}
	}

	handleCursorMoveEvent(e: MouseEvent) {
		const state = this.state;
		if (!state) return;

		let cursor = this.cursor;
		cursor.x_prev = cursor.x;
		cursor.y_prev = cursor.y;
		cursor.x = e.clientX * state.renderer.display_scale;
		cursor.y = e.clientY * state.renderer.display_scale;
		cursor.x_norm = e.clientX / window.innerWidth;
		cursor.y_norm = e.clientY / window.innerHeight;

		let canvas = state.renderer.params.canvas;

		let boundary = state.map.boundary;
		let scrolling = state.map.scrolling;

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

		state.client.socketSendCursorPos(smooth ? cursor.canvas_x_smooth : cursor.canvas_x, smooth ? cursor.canvas_y_smooth : cursor.canvas_y);

		this.room_screen_globals.setMousePosText(<span className={style.cursor_pos} > {"X " + cursor.canvas_x} < br /> {"Y " + cursor.canvas_y} </span>);

		if (cursor.down_right) {
			//Scroll
			scrolling.x += (cursor.x - cursor.x_prev) / scrolling.zoom;
			scrolling.y += (cursor.y - cursor.y_prev) / scrolling.zoom;
			this.needs_boundaries_update = true;
		}

		state.map.triggerRerender();
		cursor.just_pressed_down = false;
	}

	handleCursorDownEvent(e: MouseEvent) {
		const state = this.state;
		if (!state) return;

		this.cursor.just_pressed_down = true;

		if (e.button == 0) { // Left
			if (state.map.getZoom() < 1.0) {
				state.map.setZoom(1.0);
				this.needs_boundaries_update = true;
			}
			else {
				this.cursor.down_left = true;
				state.client.socketSendCursorDown();
			}
		}

		if (e.button == 1) {
			this.actionColorPick();
		}

		if (e.button == 2) { // Right
			this.cursor.down_right = true;
		}
	}

	handleCursorUpEvent(e: MouseEvent) {
		const state = this.state;
		if (!state) return;

		if (e.button == 0) { // Left
			this.cursor.down_left = false;
			state.client.socketSendCursorUp();
		}

		if (e.button == 2) { // Right
			this.cursor.down_right = false;
		}
	}

	handleWheelEvent(e: WheelEvent) {
		if (!this.state) return;
		let zoom_diff = -e.deltaY * 0.0015;
		this.state.map.addZoom(zoom_diff);
		this.needs_boundaries_update = true;
	}

	handleBlurEvent(_e: FocusEvent) {
		const state = this.state;
		if (!state) return;

		this.cursor.down_left = false;
		state.client.socketSendCursorUp();
	}

	selectTool(tool_id: tool.ToolID) {
		const state = this.state;
		if (!state) return;

		this.cursor.tool_id = tool_id;
		state.client.socketSendToolType(tool_id);
	}

	updateBoundary() {
		const state = this.state;
		if (!state) return;

		if (!this.needs_boundaries_update)
			return;

		this.needs_boundaries_update = false;
		state.client.socketSendBoundary();
		this.updatePreviewSystem();
	}

	private updatePreviewSystem() {
		const state = this.state;
		if (!state) return;

		//Calculate preview visibility
		let camera_zoom = state.map.getZoom();
		let target_zoom: number = 0;
		const zoom_levels = [8, 7, 6, 5, 4, 3, 2, 1];
		const thresholds = zoom_levels.map(level => Math.pow(0.5, level));
		let request_sent_count = 0;

		for (let i = 0; i < thresholds.length; i++) {
			if (camera_zoom < thresholds[i]) {
				target_zoom = zoom_levels[i];
				break;
			}
		}
		//console.log("target zoom " + target_zoom);

		for (let zoom = 1; zoom <= PREVIEW_SYSTEM_LAYER_COUNT; zoom++) {
			let layer = this.preview_system.getOrCreateLayer(zoom);
			let boundary = state.map.getPreviewBoundaries(layer.zoom);

			// if zoom matches target zoom
			if (layer.zoom == target_zoom) {
				if (request_sent_count < 30) {
					for (let y = boundary.start_y; y < boundary.end_y; y++) {
						for (let x = boundary.start_x; x < boundary.end_x; x++) {
							let preview = layer.getPreview(x, y);
							if (preview != null) {
								// already loaded, do nothing
								continue;
							}

							preview = layer.getOrCreatePreview(x, y);
							state.client.socketSendPreviewRequest(x, y, layer.zoom);
							request_sent_count++;
						}
					}
				}

				continue; // !
			}

			//normal operation
			layer.iterPreviewsInBoundary(boundary, (preview) => {
				preview.remove_timeout++;
				if (preview.remove_timeout > 10) {
					layer.removePreview(preview.x, preview.y);
				}
			});
		}

		if (request_sent_count > 0) {
			//console.log("sent", request_sent_count, "chunk preview requests");
			this.needs_boundaries_update = true;
		}
	}

	updateUserList() {
		this.callback_user_update?.()
	}

	getUserList(): Array<string> {
		const state = this.state;
		if (!state) return [];

		let arr: string[] = ["You"];

		state.client.users.forEach((user: User) => {
			if (!user) return;
			arr.push(user.nickname);
		});

		return arr;
	}

	setProcessingStatusText(text: string) {
		this.room_screen_globals.setProcessingStatusText(text);
	}
}


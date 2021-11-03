function clamp(num, min, max) {
	return num <= min ? min : num >= max ? max : num;
}

function getColorSelector() {
	return document.getElementById("mp_color_selector");
}

function dec2hex(n) {
	if (n < 0) n = 0;
	if (n > 255) n = 255;
	return n.toString(16).padStart(2, '0');
}

function rgb2hex(red, green, blue) {
	return '#' + dec2hex(red) + dec2hex(green) + dec2hex(blue);
}

const ToolID = {
	brush: 0,
	floodfill: 1
}

class Cursor {
	constructor() {
		this.just_pressed_down = false;
		this.x = 0.0;
		this.y = 0.0;
		this.x_prev = 0.0;
		this.y_prev = 0.0;
		this.canvas_x = 0.0;
		this.canvas_x_smooth = 0.0;
		this.canvas_y = 0.0;
		this.canvas_y_smooth = 0.0;
		this.down_left = false;
		this.down_right = false;
		this.brush_size = 1;
		this.tool_id = ToolID.brush;
	}
}

class Multipixel {
	client;//class Client
	map;//class Map
	chat;//class Chat
	renderer;//class Renderer
	cursor;//class Cursor
	needs_boundaries_update;//bool
	timestep;//class

	constructor(host, nick, done_callback) {
		this.client = new Client(this, host, nick, () => {
			this.onConnect(done_callback);
		});
	}

	onConnect = function (done_callback) {
		this.initRenderer();
		this.initMap();
		this.initCursor();
		this.initChat();
		this.initGUI();
		this.initListeners();
		this.initTimestep();

		//Start rendering
		this.draw();

		done_callback();
	}

	draw = function () {
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

	initRenderer = function () {
		this.renderer = new RenderEngine(document.getElementById("canvas_render"));
	}

	initMap = function () {
		this.map = new Map(this);
	}

	initCursor = function () {
		this.cursor = new Cursor();
	}

	initChat = function () {
		this.chat = new Chat(this.client);
	}

	initGUI = function () {
		let slider = document.getElementById("mp_slider_brush_size");
		slider.value = 1;
		slider.addEventListener("change", () => {
			let size = slider.value;
			this.getCursor().brush_size = size;
			this.client.socketSendBrushSize(size);
		});

		document.getElementById("button_zoom_1_1").addEventListener("click", () => {
			this.map.setZoom(1.0);
			this.map.triggerRerender();
		});

		let button_undo = document.getElementById("button_undo");
		button_undo.addEventListener("click", () => {
			this.client.socketSendUndo();
		});

		let button_tool_brush = document.getElementById("button_tool_brush");
		button_tool_brush.addEventListener("click", () => {
			this.selectTool(ToolID.brush);
			this.markSelectedTool(button_tool_brush);
		});

		let button_tool_floodfill = document.getElementById("button_tool_floodfill");
		button_tool_floodfill.addEventListener("click", () => {
			this.selectTool(ToolID.floodfill);
			this.markSelectedTool(button_tool_floodfill);
		});

		let color_history1 = document.getElementById("color_history1");
		color_history1.addEventListener("click", () => {
			this.handleColor(color_history1);
		});

		let color_history2 = document.getElementById("color_history2");
		color_history2.addEventListener("click", () => {
			this.handleColor(color_history2);
		});

		document.getElementById("mp_color_selector").addEventListener("change", () => {
			this.colorChange();
		});
	}

	initListeners = function () {
		setInterval(() => { this.updateBoundary() }, 200);

		setInterval(() => { this.client.socketSendPing() }, 8000);

		let canvas = this.renderer.getCanvas();
		let body = document.getElementById("body");

		canvas.addEventListener("mousemove", (e) => {
			let cursor = this.getCursor();
			cursor.x_prev = cursor.x;
			cursor.y_prev = cursor.y;
			cursor.x = e.clientX;
			cursor.y = e.clientY;

			let canvas = this.renderer.getCanvas();

			let boundary = this.map.boundary;
			let scrolling = this.map.scrolling;

			let raw_x = boundary.center_x - boundary.width / 2.0 + (cursor.x / canvas.width) * boundary.width;
			let raw_y = boundary.center_y - boundary.height / 2.0 + (cursor.y / canvas.height) * boundary.height;

			cursor.canvas_x = Math.floor(raw_x);
			cursor.canvas_y = Math.floor(raw_y);

			let smooth = false;
			let smooth_val;

			if (this.cursor.down_left && this.cursor.tool_id == ToolID.brush) {
				let value = document.getElementById("mp_slider_brush_smoothing").value;
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

		canvas.addEventListener("mousedown", (e) => {
			let cursor = this.getCursor();
			cursor.just_pressed_down = true;
			if (e.button == 0) {//Left
				cursor.down_left = true;
				this.client.socketSendCursorDown();
			}
			if (e.button == 1) {
				this.performColorPick();
			}
			if (e.button == 2) {//Right
				cursor.down_right = true;
			}
		});


		canvas.addEventListener("mouseup", (e) => {
			let cursor = this.getCursor();
			if (e.button == 0) {//Left
				cursor.down_left = false;
				this.client.socketSendCursorUp();
			}
			if (e.button == 2) {//Right
				cursor.down_right = false;
			}
		});

		window.addEventListener("blur", (e) => {
			let cursor = this.getCursor();
			e;
			cursor.down_left = false;
			this.client.socketSendCursorUp();
		});

		canvas.addEventListener("wheel", (e) => {
			let zoom_diff = clamp(-e.deltaY * 100.0, -1, 1) * 0.2;
			this.map.addZoom(zoom_diff, true);
			this.needs_boundaries_update = true;
		});

		body.addEventListener("contextmenu", (e) => {
			e.preventDefault();
			return false;
		});
	}

	initTimestep = function () {
		this.timestep = new Timestep();
		this.timestep.setRate(60.0);
	}

	getRenderer = function () {
		return this.renderer;
	}

	getCursor = function () {
		return this.cursor;
	}

	refreshPlayerList = function () {
		let player_list = document.getElementById("mp_player_list");
		let buf = "[Online players]<br><br>";

		let self_shown = false;
		let t = this;
		function add_self() {
			self_shown = true;
			buf += "You [" + t.client.id + "]<br>";
		}

		this.client.users.forEach(user => {
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

	selectTool = function (tool_id) {
		this.cursor.tool_id = tool_id;
		this.client.socketSendToolType(tool_id);
	}

	markSelectedTool = function (selected_element) {
		let elements = document.getElementsByClassName("button_tool");
		for (let element of elements) {
			element.classList.remove("button_tool_selected");
		}

		selected_element.classList.add("button_tool_selected");
	}

	updateBoundary = function () {
		if (!this.needs_boundaries_update)
			return;
		this.needs_boundaries_update = false;
		this.client.socketSendBoundary();
	}

	performColorPick = function () {
		let cursor = this.getCursor();
		let rgb = this.map.getPixel(cursor.canvas_x, cursor.canvas_y);
		if (rgb) {
			getColorSelector().value = rgb2hex(rgb[0], rgb[1], rgb[2]);
			this.colorChange();
		}
	}

	currentColorUpadate = function (color_string) {
		let red = parseInt("0x" + color_string.substring(1, 3));
		let green = parseInt("0x" + color_string.substring(3, 5));
		let blue = parseInt("0x" + color_string.substring(5, 7));
		this.client.socketSendBrushColor(red, green, blue);
		let cl = document.getElementsByClassName("cl");
		for (let i = cl.length - 1; i > 0; i--) {
			let color = cl[i - 1].getAttribute("contained-color");
			cl[i].style.backgroundColor = color;
			cl[i].setAttribute("contained-color", color);
		}
		cl[0].style.backgroundColor = color_string;
		cl[0].setAttribute("contained-color", color_string);
	}

	colorChange = function () {
		let selector = getColorSelector();
		let string_value = selector.value;
		this.currentColorUpadate(string_value);
	}

	handleColor = function (elem) {
		let color = elem.getAttribute("contained-color");
		if (color != null) {
			this.currentColorUpadate(color);
		}
	}

	tick = function () {
		this.map.tick();
	}
}
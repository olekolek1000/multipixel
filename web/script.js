
function clamp(num, min, max) {
	return num <= min ? min : num >= max ? max : num;
}

let mouse = {
	x: 0,
	y: 0,
	x_prev: 0,
	y_prev: 0,
	canvas_x: 0,
	canvas_y: 0,
	down_left: false,
	down_right: false
}

let needs_boundaries_update = false;

function boundary_update() {
	if (!needs_boundaries_update)
		return;
	needs_boundaries_update = false;
	client.socketSendBoundary();
}

function initListeners() {
	setInterval(boundary_update, 200);

	let viewport = getViewport();
	let body = getBody();

	viewport.addEventListener("mousemove", function (e) {
		mouse.x_prev = mouse.x;
		mouse.y_prev = mouse.y;
		mouse.x = e.clientX;
		mouse.y = e.clientY;

		let scrolling = map.getScrolling();

		let raw_x = (mouse.x - scrolling.x) / scrolling.zoom;
		let raw_y = (mouse.y - scrolling.y) / scrolling.zoom;

		mouse.canvas_x = Math.floor(raw_x);
		mouse.canvas_y = Math.floor(raw_y);

		//if (raw_x < 0) mouse.canvas_x++;
		//if (raw_y < 0) mouse.canvas_y++;

		client.socketSendCursorPos(mouse.canvas_x, mouse.canvas_y);

		if (mouse.down_right) {
			//Scroll
			scrolling.x += (mouse.x - mouse.x_prev);
			scrolling.y += (mouse.y - mouse.y_prev);
			needs_boundaries_update = true;

			map.triggerRerender();
		}
	});

	viewport.addEventListener("mousedown", function (e) {
		if (e.button == 0) {//Left
			mouse.down_left = true;
			client.socketSendCursorDown();
		}
		if (e.button == 1) {
			performColorPick();
		}
		if (e.button == 2) {//Right
			mouse.down_right = true;
		}
	});


	viewport.addEventListener("mouseup", function (e) {
		if (e.button == 0) {//Left
			mouse.down_left = false;
			client.socketSendCursorUp();
		}
		if (e.button == 2) {//Right
			mouse.down_right = false;
		}
	});

	window.addEventListener("blur", function (e) {
		e;
		mouse.down_left = false;
		client.socketSendCursorUp();
	});

	viewport.addEventListener("wheel", function (e) {
		let zoom_diff = clamp(-e.deltaY * 100.0, -1, 1) * 0.2;
		setZoom(zoom_diff, true);
		needs_boundaries_update = true;
	});

	body.addEventListener("contextmenu", function (e) {
		e.preventDefault();
		return false;
	});
}

function initRenderer() {
	function draw() {
		map.draw();
		window.requestAnimationFrame(draw);
	}

	window.requestAnimationFrame(draw);
}


function refreshPlayerList() {
	let player_list = document.getElementById("multipixel_player_list");
	let buf = "[Online players]<br><br>";

	let self_shown = false;
	add_self = function () {
		self_shown = true;
		buf += "You [" + client.id + "]<br>";
	}

	client.users.forEach(user => {
		if (user == null)
			return;

		if (!self_shown && client.id < user.id) {
			add_self();
			self_shown = true;
		}

		buf += user.nickname + " [" + user.id + "]<br>";
	})

	if (!self_shown)
		add_self();

	player_list.innerHTML = buf;
}

function handleBrushSizeSlider() {
	let sld = document.getElementById('slider');
	if (sld.className.includes('active')) {
		sld.className = 'multipixel_floating_button'
		sld.setAttribute("disabled", "disabled")
	}
	else {
		sld.className = 'active multipixel_floating_button'
		sld.removeAttribute("disabled")
		setTimeout(function () {
			sld.className = 'multipixel_floating_button'
			sld.setAttribute("disabled", "disabled")
		}, 5000)

	}
	document.getElementById('brush-size-text').innerText = sld.value + 'px';
}

function setZoom(val, add) {
	if (add && add == true) {
		map.addZoom(val);
	}
	else {
		map.setZoom(val);
	}
}

var slider_value_callback;

function handleColor(elem) {

	let color = elem.getAttribute("contained-color");
	if (color != null) {
		currentColorUpadate(color)
	}
}
function colorRefresh(parent, colr_string) {
	elem.setAttribute("contained-color", color_string);
	elem.style.backgroundColor = color_string
}

function dec2hex(n) {
	if (n < 0) n = 0;
	if (n > 255) n = 255;
	return n.toString(16).padStart(2, '0');
}

function rgb2hex(red, green, blue) {
	return '#' + dec2hex(red) + dec2hex(green) + dec2hex(blue);
}

function currentColorUpadate(color_string) {
	let red = parseInt("0x" + color_string.substring(1, 3))
	let green = parseInt("0x" + color_string.substring(3, 5))
	let blue = parseInt("0x" + color_string.substring(5, 7))
	client.socketSendBrushColor(red, green, blue)
	let cl = document.getElementsByClassName("cl");
	for (let i = cl.length - 1; i > 0; i--) {
		let color = cl[i - 1].getAttribute("contained-color");
		cl[i].style.backgroundColor = color
		cl[i].setAttribute("contained-color", color)
	}
	cl[0].style.backgroundColor = color_string;
	cl[0].setAttribute("contained-color", color_string)
}

function getColorSelector() {
	return document.getElementById("multipixel_color_selector");
}

function colorChange() {
	let selector = getColorSelector();
	let string_value = selector.value;
	currentColorUpadate(string_value)
}

function performColorPick() {
	let rgb = map.getPixel(mouse.canvas_x, mouse.canvas_y);
	if (rgb) {
		getColorSelector().value = rgb2hex(rgb[0], rgb[1], rgb[2]);
		colorChange();
	}
}
function getById(id) {
	return document.getElementById(id)
}

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

let body = getById("body");
let viewport = getById("viewport");

let needs_boundaries_update = false;

function boundary_update() {
	if (!needs_boundaries_update)
		return;
	needs_boundaries_update = false;
	client.socketSendBoundary();
}

function initListeners() {
	setInterval(boundary_update, 200);

	viewport.addEventListener("mousemove", function (e) {
		mouse.x_prev = mouse.x;
		mouse.y_prev = mouse.y;
		mouse.x = e.clientX;
		mouse.y = e.clientY;

		let scrolling = map.getScrolling();

		mouse.canvas_x = (mouse.x - scrolling.x) / scrolling.zoom;
		mouse.canvas_y = (mouse.y - scrolling.y) / scrolling.zoom;

		client.socketSendCursorPos(mouse.canvas_x, mouse.canvas_y);

		if (mouse.down_right) {
			//Scroll
			scrolling.x += (mouse.x - mouse.x_prev);
			scrolling.y += (mouse.y - mouse.y_prev);
			needs_boundaries_update = true;

			triggerRerender();
		}
	});

	viewport.addEventListener("mousedown", function (e) {
		if (e.button == 0) {//Left
			mouse.down_left = true;
			client.socketSendCursorDown();
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
		if (needs_redraw) {
			map.draw();
			needs_redraw = false;
		}
		window.requestAnimationFrame(draw);
	}

	window.requestAnimationFrame(draw);
}


function refreshPlayerList() {
	let player_list = getById("player_list");
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
		sld.className = 'floating-button'
		sld.setAttribute("disabled", "disabled")
	}
	else {
		sld.className = 'active floating-button'
		sld.removeAttribute("disabled")
		setTimeout(function () {
			sld.className = 'floating-button'
			sld.setAttribute("disabled", "disabled")
		}, 5000)

	}
	document.getElementById('brush-size-text').innerText = sld.value + 'px';
}



function handleZoomSlider() {
	let sld = document.getElementById('zoom-level-range');
	if (sld.className.includes('active')) {
		sld.className = 'floating-button'
		sld.setAttribute("disabled", "disabled")
	}
	else {
		sld.className = 'active floating-button'
		sld.removeAttribute("disabled")
		setTimeout(function () {
			sld.className = 'floating-button'
			sld.setAttribute("disabled", "disabled")
		}, 5000)

	}
}

function sliderValueToZoom(num) {
	num = parseFloat(num);
	return 0.1 * (639.0 * Math.pow(num, 4.0) + 1);
}

function zoomToSliderValue(num) {
	num = parseFloat(num);
	return (Math.pow(10 * num - 1, 1.0 / 4.0)) / (Math.sqrt(3) * Math.pow(71, 1.0 / 4.0));
}

function setZoom(val, add) {
	if (add) {
		map.addZoom(val);
	}
	else {
		map.setZoom(val);
	}

	document.getElementById('zoom-level-number').value = map.scrolling.zoom;
	document.getElementById('zoom-level-range').value = zoomToSliderValue(map.scrolling.zoom);
}

var slider_value_callback;

function updateSliderZoomValue() {
	let slider = document.getElementById("zoom-level-range");
	setZoom(sliderValueToZoom(slider.value), false);
}

function handleSliderZoomDown() {
	if (slider_value_callback != null)
		clearInterval(slider_value_callback);

	slider_value_callback = null;
	slider_value_callback = setInterval(updateSliderZoomValue, 15);
}

function handleSliderZoomUp() {
	clearInterval(slider_value_callback);
	slider_value_callback = null;
	updateSliderZoomValue();
	needs_boundaries_update = true;
}
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
function colorChange() {
	let selector = document.getElementById('color')
	let string_value = selector.value;
	currentColorUpadate(string_value)
}
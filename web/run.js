var client;
var map;
var renderer;

function showLoginScreen() {
	document.getElementById("login_screen").style.visibility = "visible";
	document.getElementById("mp_screen").style.visibility = "hidden";
}

function showMultipixelScreen() {
	document.getElementById("login_screen").style.visibility = "hidden";
	document.getElementById("mp_screen").style.visibility = "visible";
}

window.onload = function () {
	showLoginScreen();
}

//Called after connecting to the server
function onConnect() {
	initRenderer();
	map = new Map();

	let chat = new Chat(client);

	let slider = document.getElementById("mp_slider_brush_size");
	slider.value = 1;
	slider.addEventListener("change", () => {
		let size = slider.value;
		mouse.brush_size = size;
		client.socketSendBrushSize(size);
	});

	document.getElementById("button_zoom_1_1").addEventListener("click", () => {
		map.setZoom(1.0);
		map.triggerRerender();
	});

	initListeners();
	setInterval(() => { client.socketSendPing() }, 8000);

	showMultipixelScreen();
}

function onStartClick() {
	let nick = document.getElementById("nick").value.trim()
	if (nick.length == 0) {
		document.getElementById("nick").value = ""
		document.getElementById("nick").setAttribute("placeholder", "Nickname cannot be empty");
	}
	else if (nick.length > 32) {
		document.getElementById("nick").value = ""
		document.getElementById("nick").setAttribute("placeholder", "Nickname length cannot exceed 32 characters(bytes).");
	}
	else {
		document.getElementById("logo").outerHTML = ""
		client = new Client("wss://oo8dev.com/ws_multipixel/", nick, onConnect);
	}
}
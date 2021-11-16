import { Multipixel } from "./multipixel";

var map;
var renderer;

function showLoginScreen() {
	document.getElementById("login_screen").style.visibility = "visible";
	document.getElementById("mp_screen").style.visibility = "hidden";

	(document.getElementById("nick") as HTMLInputElement).value = localStorage.getItem("nick");
}

function showMultipixelScreen() {
	document.getElementById("login_screen").style.visibility = "hidden";
	document.getElementById("mp_screen").style.visibility = "visible";
}

window.onload = function () {
	showLoginScreen();
}

document.getElementById("button_start").onclick = onStartClick;

function onStartClick() {
	let nick_element = (document.getElementById("nick") as HTMLInputElement);
	let nick = nick_element.value.trim();

	localStorage.setItem("nick", nick);
	if (nick.length == 0) {
		nick_element.value = ""
		nick_element.setAttribute("placeholder", "Nickname cannot be empty");
	}
	else if (nick.length > 32) {
		nick_element.value = ""
		nick_element.setAttribute("placeholder", "Nickname length cannot exceed 32 characters(bytes).");
	}
	else {
		let multipixel = new Multipixel("wss://kuczaracza.com/ws_multipixel/", nick, () => {
			showMultipixelScreen();
		});
	}
}
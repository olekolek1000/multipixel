import { Multipixel } from "./multipixel";

function showLoginScreen() {
	document.getElementById("login_screen").style.visibility = "visible";
	document.getElementById("mp_screen").style.visibility = "hidden";

	let nick_name_element = (document.getElementById("nickname") as HTMLInputElement);
	let room_name_element = (document.getElementById("room_name") as HTMLInputElement);

	nick_name_element.value = localStorage.getItem("nickname");
	room_name_element.value = localStorage.getItem("room_name");

	if (room_name_element.value.length == 0)
		room_name_element.value = "main";
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
	let nick_name_element = (document.getElementById("nickname") as HTMLInputElement);
	let room_name_element = (document.getElementById("room_name") as HTMLInputElement);

	let nickname = nick_name_element.value.trim();
	let room_name = room_name_element.value.trim().toLowerCase();

	localStorage.setItem("nickname", nickname);
	localStorage.setItem("room_name", room_name);

	if (nickname.length == 0) {
		nick_name_element.value = ""
		nick_name_element.setAttribute("placeholder", "Nickname cannot be empty");
	}
	else if (nickname.length > 32) {
		nick_name_element.value = ""
		nick_name_element.setAttribute("placeholder", "Nickname length cannot exceed 32 characters(bytes).");
	}
	else if (room_name.length < 3) {
		room_name_element.value = "";
		room_name_element.setAttribute("placeholder", "Too short room name");
	}
	else if (room_name.length > 32) {
		room_name_element.value = "";
		room_name_element.setAttribute("placeholder", "Too long room name");
	}
	else {
		let multipixel = new Multipixel("ws://127.0.0.1:59900", nickname, room_name, () => {
			showMultipixelScreen();
		});
	}
}
import { Multipixel } from "./multipixel";

function getLoginScreen() {
	return document.getElementById("login_screen") as HTMLElement;
}

function getMultipixelScreen() {
	return document.getElementById("mp_screen") as HTMLElement;
}

function showLoginScreen() {
	getLoginScreen().style.visibility = "visible";
	getMultipixelScreen().style.visibility = "hidden";

	let nick_name = document.getElementById("nickname") as HTMLInputElement;
	nick_name.value = localStorage.getItem("nickname") as string;

	let room_name = document.getElementById("room_name") as HTMLInputElement;
	room_name.value = localStorage.getItem("room_name") as string;

	if (room_name.value.length == 0)
		room_name.value = "main";

	let button_start = document.getElementById("button_start") as HTMLElement;
	button_start.addEventListener("click", onStartClick);
}

function showMultipixelScreen() {
	getLoginScreen().style.visibility = "hidden";
	getMultipixelScreen().style.visibility = "visible";
}

window.onload = function () {
	showLoginScreen();
}


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

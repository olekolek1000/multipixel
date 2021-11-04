
var map;
var renderer;

function showLoginScreen() {
	document.getElementById("login_screen").style.visibility = "visible";
	document.getElementById("mp_screen").style.visibility = "hidden";

	document.getElementById("nick").value = localStorage.getItem("nick");
}

function showMultipixelScreen() {
	document.getElementById("login_screen").style.visibility = "hidden";
	document.getElementById("mp_screen").style.visibility = "visible";
}

window.onload = function () {
	showLoginScreen();
}


function onStartClick() {
	let nick = document.getElementById("nick").value.trim()
	localStorage.setItem("nick", nick);
	if (nick.length == 0) {
		document.getElementById("nick").value = ""
		document.getElementById("nick").setAttribute("placeholder", "Nickname cannot be empty");
	}
	else if (nick.length > 32) {
		document.getElementById("nick").value = ""
		document.getElementById("nick").setAttribute("placeholder", "Nickname length cannot exceed 32 characters(bytes).");
	}
	else {
		let multipixel = new Multipixel("wss://olekolek1000.com/ws_multipixel/", nick, () => {
			showMultipixelScreen();
		});
	}
}
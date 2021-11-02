
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
		let multipixel = new Multipixel("wss://oo8dev.com/ws_multipixel/", nick, () => {
			showMultipixelScreen();
		});
	}
}
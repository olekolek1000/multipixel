var client;

window.onload = function () {
	document.getElementById("nick-button").removeAttribute("disabled")
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
		client = new Client("wss://oo8dev.com/ws_multipixel/", nick, function () {
			initListeners();
			initRenderer();
			setInterval(() => { client.socketSendPing() }, 8000);
		});
	}
}

class Chat {

	constructor(client) {
		this.client = client;
		this.client.setChatObject(this);

		this.chat_input = document.getElementById("mp_chat_input");
		this.chat_input.value = "";

		this.chat_history = document.getElementById("mp_chat_history");


		this.chat_input.addEventListener("keypress", (e) => {
			if (e.key == "Enter") {
				if (this.chat_input.value.length > 0) {
					this.client.socketSendMessage(this.chat_input.value);
				}
				this.chat_input.value = "";
			}
		});

		this.chat_history.append()
	}

	addMessage = function (str) {
		let chat_message = document.createElement("div");
		chat_message.classList.add("mp_chat_message");
		chat_message.innerText = str;
		chat_message.style.opacity = 0.0;
		let this_removed = false;

		this.chat_history.appendChild(chat_message);

		let anim_size = 1.0;
		let interval_anim = setInterval(() => {
			anim_size *= 0.85;
			if (anim_size < 0.01) {
				chat_message.style.transform = "";
				chat_message.style.opacity = 1.0;
				clearInterval(interval_anim);
			}
			chat_message.style.transform = "translateY(" + (anim_size * 16.0) + "px)";
			chat_message.style.opacity = 1.0 - anim_size;
		}, 16);

		setTimeout(() => {
			if (this_removed) return;
			let opacity = 1.0;
			let interval = setInterval(() => {
				if (this_removed) {
					clearInterval(interval);
					return;
				}
				opacity *= 0.9;
				chat_message.style.opacity = opacity;

				let padding = (opacity * 4.0) + "px";
				let margin = (1.0 - opacity) * -8.0 + "px";
				chat_message.style.paddingTop = padding;
				chat_message.style.paddingBottom = padding;
				chat_message.style.marginTop = margin;
				chat_message.style.marginBottom = margin;

				if (opacity < 0.01) {
					clearInterval(interval);
					this.chat_history.removeChild(chat_message);
					chat_message.remove();
				}
			}, 33);
		}, 20000);
	}
}
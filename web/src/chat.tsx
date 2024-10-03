import { Client } from "./client";
import React, { useEffect, useRef, useState } from "react";
import style_chat from "./chat.scss";
import { TextField } from "./gui_custom";

class ChatLine {
	message!: string;
	html_mode!: boolean;
	key!: number;
}

export class Chat {
	client: Client;
	chat_history: Array<ChatLine>;
	setHistory: any;
	ref_history?: React.RefObject<HTMLInputElement>;
	message_index: number = 0;

	constructor(client: Client) {
		this.chat_history = new Array();
		this.client = client;
		this.client.setChatObject(this);
	}

	addMessage(str: string, html_mode: boolean) {
		this.chat_history.push({
			message: str,
			html_mode: html_mode,
			key: this.message_index++
		});

		// max 100 messages at once
		if (this.chat_history.length > 100) {
			this.chat_history.splice(0);
		}

		if (this.setHistory) {
			this.setHistory(this.chat_history.map((cell) => {
				let el;

				if (cell.html_mode) {
					el = <div key={cell.key} className={style_chat.chat_message} dangerouslySetInnerHTML={{ __html: cell.message }} />
				}
				else {
					el = <div key={cell.key} className={style_chat.chat_message}>
						{cell.message}
					</div>;
				}

				return el;
			}));
		}
	}
}

export function ChatRender({ chat }: { chat?: Chat }) {
	if (!chat) {
		return <></>;
	}

	const [text, setText] = useState("");
	const [history, setHistory] = useState<JSX.Element>(<></>);
	const ref_history = useRef<HTMLInputElement>(null);
	chat.setHistory = setHistory;
	chat.ref_history = ref_history;

	useEffect(() => {
		if (ref_history.current) {
			let div = ref_history.current! as HTMLDivElement;
			div.scrollTop = div.scrollHeight;
		}
	}, [history]);

	if (!chat) {
		return <></>;
	}

	let messages = undefined;

	if (chat.chat_history.length > 0) {
		messages = <div ref={ref_history} className={style_chat.chat_history}>
			{history}
		</div>
	}

	return <div className={style_chat.chat_box}>
		{messages}
		<div className={style_chat.chat_input}>
			<TextField valfunc={[text, setText]} onReturnPress={() => {
				if (text.length > 0) {
					chat.client.socketSendMessage(text);
				}
				setText("");
			}} />
		</div>
	</div>
}
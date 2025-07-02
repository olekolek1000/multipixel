import { ChatMessageType, Client } from "./client";
import React, { type ReactNode, useEffect, useRef, useState } from "react";
import style_chat from "./chat.module.scss";
import { TextField } from "./gui_custom";
import bbobHTML from '@bbob/html'
import presetHTML5 from '@bbob/preset-html5'

class ChatLine {
	message!: string;
	type!: ChatMessageType;
	key!: number;
}

function StylizedText({ text }: { text: string }) {
	let processed = bbobHTML(text, presetHTML5(), {
		onlyAllowTags: ["color", "b", "i", "u", "s"]
	});

	return <span style={{ whiteSpace: "pre-line" }} dangerouslySetInnerHTML={{ __html: processed }}></span>
}

export class Chat {
	client: Client;
	chat_history: Array<ChatLine>;
	setHistory: any;
	ref_history?: React.RefObject<HTMLDivElement | null>;
	message_index: number = 0;

	constructor(client: Client) {
		this.chat_history = new Array();
		this.client = client;
		this.client.setChatObject(this);
	}

	addMessage(str: string, type: ChatMessageType) {
		this.chat_history.push({
			message: str,
			type: type,
			key: this.message_index++
		});

		// max 100 messages at once
		while (this.chat_history.length > 100) {
			this.chat_history.shift();
		}

		if (this.setHistory) {
			this.setHistory(this.chat_history.map((cell) => {
				let msg: ReactNode;

				if (cell.type == ChatMessageType.stylized) {
					msg = <StylizedText text={cell.message} />
				}
				else {
					msg = cell.message;
				}

				return <div key={cell.key} className={style_chat.chat_message}>
					{msg}
				</div>
			}));
		}
	}
}



export function ChatRender({ chat }: { chat?: Chat }) {
	if (!chat) {
		return <></>;
	}

	const [text, setText] = useState("");
	const [history, setHistory] = useState<ReactNode>(<></>);
	const ref_history = useRef<HTMLDivElement | null>(null);
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
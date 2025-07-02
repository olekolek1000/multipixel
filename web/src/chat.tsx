import { ChatMessageType, Client } from "./client";
import { useEffect, useRef, useState } from "react";
import style_chat from "./chat.module.scss";

import { ChatInput } from "./chat/ChatInput";
import { Store, useStore } from "@tanstack/react-store";
import { ChatMessage } from "./chat/ChatMessage";

export interface ChatLine {
	message: string
	type: ChatMessageType
	localChatIndex: number
}


export class Chat {
	client: Client;
	chat_history: Array<ChatLine>;
	messageLastIndex: number = 0;

	// this is state than can be sucscribed to in react components
	chatHistoryState: Store<ChatLine[]>;

	constructor(client: Client) {
		this.chatHistoryState = new Store<ChatLine[]>([]);
		this.chat_history = [];
		this.client = client;
		this.client.setChatObject(this);
	}

	addMessage(str: string, type: ChatMessageType) {
		this.chatHistoryState.setState((prev) => [
			...prev,
			{
				message: str,
				type: type,
				localChatIndex: this.messageLastIndex++
			}
		]);
	}
}


export function ChatRender({ chat }: { chat?: Chat }) {
	if (!chat)
		throw new Error("ChatRender: Tried to render chat without chat object");

	const [chatInput, setChatInput] = useState("");
	const chatHistory = useStore(chat.chatHistoryState);
	
	const ref_history = useRef<HTMLDivElement | null>(null);
	
	useEffect(() => {
		if (ref_history.current) {
			let div = ref_history.current! as HTMLDivElement;
			div.scrollTop = div.scrollHeight;
		}
	}, [chatHistory]);

	return (
		<div className={style_chat.chat_box}>

 			<div ref={ref_history} className={style_chat.chat_history}>	
				{chatHistory.map((line) => 
					<ChatMessage key={line.localChatIndex} textLine={line}/>
				)}
			</div>

			<div className={style_chat.chat_input}>
				<ChatInput
					value={chatInput}
					onChange={setChatInput}
					onReturnPress={() => {
						if (chatInput.length > 0) {
							chat.client.socketSendMessage(chatInput);
						}
						setChatInput("");
					}} 
				/>
			</div>
		</div>
	);
}

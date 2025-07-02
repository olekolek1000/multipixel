import { ChatMessageType, Client } from "../client";
import { useEffect, useRef, useState } from "react";

import { ChatInput } from "./components/ChatInput";
import { Store, useStore } from "@tanstack/react-store";
import { ChatMessage } from "./components/ChatMessage";

import chatStyles from "./chat.module.scss";

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
		<div className={chatStyles.chat_box + " min-w-xs"}>

			{!!chatHistory.length &&
				<div ref={ref_history} className={chatStyles.chat_history}>	
					{chatHistory.map((line) => 
						<ChatMessage key={line.localChatIndex} textLine={line}/>
					)}
				</div>
			}
 			
			<div >
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

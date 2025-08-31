import { ChatMessageType, Client } from "../client";
import { useEffect, useRef, useState } from "react";

import { ChatInput } from "./components/ChatInput";
import { Store, useStore } from "@tanstack/react-store";
import { ChatMessage } from "./components/ChatMessage";

import { FloatContainer } from "@/ui/components/FloatContainer";

export interface ChatLine {
	message: string
	author?: string
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

	prepareChatMessage(author: string, message: string, type: ChatMessageType): ChatLine {
		return {
			message,
			author,
			type: type,
			localChatIndex: this.messageLastIndex++,
		}
	}

	addMessage(sender: string, str: string, type: ChatMessageType) {
		this.chatHistoryState.setState((prev) => [
			...prev,
			this.prepareChatMessage(sender, str, type),
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
		if (!ref_history.current)
			return;

		let div = ref_history.current! as HTMLDivElement;
		div.scrollTop = div.scrollHeight;
	}, [chatHistory]);

	return (
		<div className="fixed bottom-2 left-2 w-full max-w-xs p-2">
			{!!chatHistory.length &&
				<FloatContainer className="w-full">
					<div ref={ref_history} className="max-h-64 flex flex-col overflow-y-scroll overflow-x-visible scroll-smooth">
						{chatHistory.map((line) =>
							<ChatMessage key={line.localChatIndex} textLine={line} />
						)}
					</div>
				</FloatContainer>
			}

			<div className="pt-2">
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

import { useState, type FC } from "react"
import { Input } from "@/ui/components/Input";
import { FloatContainer } from "@/ui/components/FloatContainer";

const PLACEHOLDERS = [
	"Type a message...",
	"Say hello!",
	"Is anyone there?",
	"Type your message here...",
	"Public chat...",
	"Chat with others...",
];

function getRandomPlaceholder() {
	return PLACEHOLDERS[Math.floor(Math.random() * PLACEHOLDERS.length)];
}

export interface ChatInputProps {
	value: string
	onChange: (value: string) => void
	onReturnPress?: () => void
}

export const ChatInput: FC<ChatInputProps> = ({ value, onChange, onReturnPress }) =>  {
	const [placeholder] = useState(getRandomPlaceholder());

	return (
		<FloatContainer className="flex p-0 mt-2 w-full">
			<Input
				className="focus:outline-none w-full py-1"
				style={{ background: "none", border: "none"}}
				placeholder={placeholder}
				value={value}
				onChange={e => onChange(e.target.value)}
				onKeyDown={(e) => {
					if (e.key == "Enter")
						onReturnPress?.();
				}} 
			/>
		</FloatContainer>
	)
}

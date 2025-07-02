import type { FC } from "react"

import styles from '../../style.module.scss';

export interface ChatInputProps {
	value: string
	onChange: (value: string) => void
	onReturnPress?: () => void
}

export const ChatInput: FC<ChatInputProps> = ({ value, onChange, onReturnPress }) =>  (
	<span className={styles.text_input_bg}>
		<input
			className={styles.text_input}
			placeholder="Type a message..."
			value={value}
			onChange={e => onChange(e.target.value)}
			onKeyDown={(e) => {
				if (e.key == "Enter")
					onReturnPress?.();
			}} 
		/>
	</span>
)

import type { FC } from "react"

import styles from '../style.module.scss';

export interface ChatInputProps {
	value: string
	onChange: (value: string) => void
	required?: boolean
	type?: React.HTMLInputTypeAttribute
	onReturnPress?: () => void
}

export const ChatInput: FC<ChatInputProps> = ({ required, value, onChange, type, onReturnPress }) =>  (
	<span className={styles.text_input_bg}>
		<input
			className={styles.text_input}
			required={required}
			value={value}
			type={type}
			onChange={e => onChange(e.target.value)}
			onKeyDown={(e) => {
				if (e.key == "Enter")
					onReturnPress?.();
			}} 
		/>
	</span>
)

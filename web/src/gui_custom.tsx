import React, { useState } from "react";
import style from "./style.scss"

export function LabeledTextField(
	{ required, label, valfunc, type, onReturnPress }:
		{
			required?: boolean,
			label: string,
			valfunc: [value: string, func: (str: string) => void],
			type?: React.HTMLInputTypeAttribute,
			error?: boolean,
			error_text?: string,
			onReturnPress?: () => void
		}) {

	return <span className={style.text_input_labeled_bg}>
		<span className={style.text_input_title}>
			<span className={style.text_input_label}>
				{label}
			</span>
		</span>
		<input
			className={style.text_input_labeled}
			required={required}
			value={valfunc[0]}
			type={type}
			onChange={e => valfunc[1](e.target.value)}
			onKeyDown={
				(e) => {
					if (e.key == "Enter" && onReturnPress)
						onReturnPress();
				}
			} />
	</span>
}

export function TextField(
	{ required, valfunc, type, onReturnPress }:
		{
			required?: boolean,
			valfunc: [value: string, func: (str: string) => void],
			type?: React.HTMLInputTypeAttribute,
			error?: boolean,
			error_text?: string,
			onReturnPress?: () => void
		}) {

	return <span className={style.text_input_bg}>
		<input
			className={style.text_input}
			required={required}
			value={valfunc[0]}
			type={type}
			onChange={e => valfunc[1](e.target.value)}
			onKeyDown={
				(e) => {
					if (e.key == "Enter" && onReturnPress)
						onReturnPress();
				}
			} />
	</span>
}

//Inline box, dir: right
export function BoxRight(
	{ children, nowrap, ref }: { children: any, nowrap?: boolean, ref?: any }
) {
	return <div style={{
		display: "flex",
		gap: "0.35cm",
		flexWrap: nowrap ? undefined : "wrap",
		alignItems: "center",
	}} ref={ref}>
		{children}
	</div>
}

//Inline box, dir: down
export function BoxDown(
	{ children, nogap, nowrap, center_horiz, center_vert }: { children: any, nogap?: boolean, nowrap?: boolean, center_horiz?: boolean, center_vert?: boolean }
) {
	return <div style={{
		display: "flex",
		gap: nogap ? undefined : "0.35cm",
		flexWrap: nowrap ? undefined : "wrap",
		flexDirection: "column",
		alignItems: center_horiz ? "center" : undefined,
		justifyContent: center_vert ? "center" : undefined
	}} >
		{children}
	</div>
}

export function TitleTiny({ children }: { children: any }) {
	return <span className={style.text_tiny}>
		{children}
	</span>
}

export function TitleSmall({ children }: { children: any }) {
	return <span className={style.text_small}>
		{children}
	</span>
}

export function Title({ children }: { children: any }) {
	return <span className={style.text_big}>
		{children}
	</span>
}


export function FormErrorText() {
	const [error_msg, setErrorMsg] = useState("");

	let msg: JSX.Element | undefined = undefined;

	if (error_msg && error_msg.length > 0) {
		msg = <span style={{
			fontWeight: "bold",
			color: "#F99",
			fontSize: "1.1em"
		}}>{error_msg}</span>
	}

	return {
		launch: async (callback: any) => {
			try {
				await callback();
				setErrorMsg("");
			}
			catch (e) {
				setErrorMsg(e ? e.toString() : "Unknown error");
			}
		},
		msg: msg,
		setErrorMsg: setErrorMsg
	}
}

export function Button({ children, on_click }: { children: any, on_click: any }) {
	return <div className={style.button} onClick={on_click}>
		{children}
	</div>
}

export function ButtonTool({ children, on_click, highlighted }: { children: any, on_click: any, highlighted?: boolean }) {
	return <div className={`${style.button_tool} ${highlighted ? style.button_tool_highlighted : ""}`} onClick={on_click}>
		{children}
	</div>
}

export function Tooltip({ children, title }: { children: any, title: any }) {
	const [hovered, setHovered] = useState(false);

	let content = undefined;

	if (hovered) {
		content = <div className={style.tooltip}>
			{title}
		</div>
	}

	return <div onMouseEnter={() => { setHovered(true); }} onMouseLeave={() => { setHovered(false); }}>
		<>
			{hovered ? <div style={{
				position: "absolute",
				right: "0",
			}}>
				{content}
			</div> : undefined}
			{children}
		</>
	</div>
}



export function Icon({ path }: { path: string }) {
	return <img src={path} className={style.icon}>
	</img>
}
import React, { JSX, useEffect, useRef, useState } from "react";
import scss from "./style.module.scss"

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

	return <span className={scss.text_input_labeled_bg}>
		<span className={scss.text_input_title}>
			<span className={scss.text_input_label}>
				{label}
			</span>
		</span>
		<input
			className={scss.text_input_labeled}
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

	return <span className={scss.text_input_bg}>
		<input
			className={scss.text_input}
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
	return <span className={scss.text_tiny}>
		{children}
	</span>
}

export function TitleSmall({ children }: { children: any }) {
	return <span className={scss.text_small}>
		{children}
	</span>
}

export function Title({ children }: { children: any }) {
	return <span className={scss.text_big}>
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
	return <div className={scss.button} onClick={on_click}>
		{children}
	</div>
}

export function ButtonTool({ children, on_click, highlighted }: { children: any, on_click: any, highlighted?: boolean }) {
	return <div className={`${scss.button_tool} ${highlighted ? scss.button_tool_highlighted : ""}`} onClick={on_click}>
		{children}
	</div>
}

export function Tooltip({ children, title }: { children: any, title: any }) {
	const [hovered, setHovered] = useState(false);

	let content = undefined;

	if (hovered) {
		content = <div className={scss.tooltip}>
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
	return <img src={path} className={scss.icon}>
	</img>
}

function mix(x: number, y: number, a: number) {
	return x * (1.0 - a) + y * a;
}

function mixInverse(x: number, y: number, a: number) {
	return (a - x) / (y - x);
}

function quantize(value: number, steps: number) {
	return Math.floor(value * (steps - 1)) / (steps - 1);
}

function getSliderParams(el: HTMLDivElement, value: number, steps?: number) {
	value = Math.max(value, 0.0);
	value = Math.min(value, 1.0);
	const rect = el.getBoundingClientRect();

	const margin = 12.0;
	const width = rect.width - margin * 2.0;
	const left = rect.x + margin;
	const right = left + width;

	return {
		handle_shift: mix(margin, rect.width - margin, steps === undefined ? value : quantize(value, steps)),
		top: rect.top,
		width: width,
		left: left,
		right: right,
		margin: margin,
	};
}


export function Slider({ title, mapped_value, setMappedValue, width, on_change, steps, min, max }: {
	title: string,
	mapped_value: number,
	setMappedValue: any,
	on_change: (value: number) => void,
	width?: number,
	steps?: number,
	min?: number,
	max?: number,
}) {
	if (min === undefined) {
		min = 0.0;
	}
	if (max === undefined) {
		max = 1.0;
	}

	const getNorm = (mapped: number) => {
		return mixInverse(min!, max!, mapped);
	}

	const getMapped = (norm: number) => {
		return mix(min!, max!, norm);
	}

	const ref_bar = useRef<HTMLDivElement | null>(null);
	const [handle_shift, setHandleShift] = useState(0.0);
	const [down, setDown] = useState(false);
	const [line_width, setLineWidth] = useState(0.0);

	useEffect(() => {
		const el = ref_bar.current;
		if (!el) {
			return;
		}

		const par = getSliderParams(el, getNorm(mapped_value), steps);
		setHandleShift(par.handle_shift);
		setLineWidth(par.width);
	}, [ref_bar]);

	const updatePos = (mouse_x: number) => {
		const el = ref_bar.current!;
		let norm_value = getNorm(mapped_value);
		const par = getSliderParams(el, norm_value);
		const rel_x = mouse_x - par.left;
		let norm_x = rel_x / (par.right - par.left);
		if (steps !== undefined) {
			norm_x += 0.5 / steps; // center handle
		}

		norm_value = norm_x;

		const mapped = getMapped(Math.max(0.0, Math.min(1.0, norm_value)));
		setMappedValue(mapped);
		on_change(mapped);

		const par2 = getSliderParams(el, norm_value, steps);
		setHandleShift(par2.handle_shift);
		setLineWidth(par2.width);
	}

	useEffect(() => {
		if (!down) {
			return;
		}

		const func_move = (e: MouseEvent) => {
			updatePos(e.clientX);
		};

		const func_up = () => {
			setDown(false);
		}

		document.addEventListener("mousemove", func_move);
		document.addEventListener("mouseup", func_up);

		return () => {
			document.removeEventListener("mousemove", func_move);
			document.removeEventListener("mouseup", func_up);
		}
	}, [down]);

	let lines: Array<JSX.Element> | undefined = undefined;

	const calc_width = width ? width : 160;

	if (steps !== undefined) {
		lines = [];
		for (let i = 0; i <= steps; i++) {
			lines.push(<div key={i} className={scss.slider_line} style={{
				left: ((i + (max - min) / 16.0) * (line_width / (steps))) + "px"
			}} />);
		}
	}

	return <div className={scss.slider_container}>
		<div className={scss.slider}
			style={{
				width: (calc_width) + "px"
			}}
			onMouseDown={(e) => {
				setDown(true);
				updatePos(e.clientX);
			}}
			onMouseMove={(e) => {
				if (!down) {
					return;
				}
				updatePos(e.clientX);
			}}
			onMouseUp={() => {
				setDown(false);
			}}
		>
			<div ref={ref_bar} className={scss.slider_bar}>

				<div className={scss.slider_filling}
					style={{
						width: handle_shift + "px"
					}}
				/>
				{lines}
				<div className={scss.slider_handle}
					style={{
						visibility: (ref_bar && ref_bar.current) ? "visible" : "hidden",
						transform: "translateX(-12px) translateY(-12px) translateX(" + handle_shift + "px)",
					}}>
				</div>
			</div>
			<div className={scss.slider_value}
				style={{
					left: handle_shift + "px"
				}}>
				{Math.floor(mapped_value)}
			</div>
		</div>
		{title}
	</div>
}
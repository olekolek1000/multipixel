import React, { useEffect, useRef, useState } from "react";
import { BoxDown, BoxRight, Icon } from "./gui_custom";
import style_toolbox from "./toolbox.scss"
import style_room from "./room_screen.scss";
import { Multipixel, rgb2hex } from "./multipixel"
import { lerp } from "./timestep";
import Picker from "vanilla-picker";

export enum ToolType {
	none,
	brush,
	spray,
	floodfill,
	blur,
}

export class ColorPaletteGlobals {
	multipixel!: Multipixel;
	toolbox_globals!: ToolboxGlobals;

	column_count: number = 0;
	row_count: number = 0;
	rows = new Array<ColorPaletteRow>();

	selected_row: number = 0;
	selected_column: number = 0;

	constructor(toolbox_globals: ToolboxGlobals, multipixel: Multipixel) {
		this.toolbox_globals = toolbox_globals;
		this.multipixel = multipixel;

		this.setColumnCount(10);
		this.setRowCount(4);

		let clr = new RGBColor();
		clr.r = 255;
		clr.g = 0;
		clr.b = 0;
		this.rows[0].color_left = clr;
		this.setSelected(0, 0, clr);
	}

	setSelected(row: number, column: number, color: RGBColor) {
		this.selected_row = row;
		this.selected_column = column;
		this.multipixel.client.socketSendBrushColor(color.r, color.g, color.b);
		this.refreshList();
	}

	setColumnCount(count: number) {
		if (count < 2) count = 2;
		if (count > 30) count = 30;
		this.column_count = count;
		this.refreshList();
	}

	setRowCount(count: number) {
		if (count < 1) count = 1;
		if (count > 20) count = 20;
		this.row_count = count;
		this.refreshList();
	}

	refreshList() {
		while (this.rows.length < this.row_count)
			this.rows.push(new ColorPaletteRow(this.rows.length));

		while (this.rows.length > this.row_count)
			this.rows.pop();

		this.toolbox_globals.setKeyPalette(this.toolbox_globals.key_palette + 1);
	}

	setColor(color: RGBColor) {
		let first = true;
		let row = this.selected_row;
		let column = 0;

		if (this.selected_column > this.column_count / 2) {
			column = this.column_count - 1;
			first = false;
		}

		if (first)
			this.rows[row].color_left = color;
		else
			this.rows[row].color_right = color;
		this.setSelected(row, column, color);
	}
}

export class ToolboxGlobals {
	multipixel!: Multipixel;
	picker?: Picker;

	tool_type: ToolType = ToolType.none;
	setToolType: any;

	param_tool_size: number = 1; //in pixels
	setToolSize: any;

	param_tool_smoothing: number = 0.0; // 0.0 - 1.0
	setToolSmoothing: any;

	param_tool_flow: number = 0.1; // 0.0 - 1.0
	setToolFlow: any;

	key_palette: number = 0;
	setKeyPalette: any;

	color_palette?: ColorPaletteGlobals;

	constructor(multipixel: Multipixel) {
		this.multipixel = multipixel;
	}
};


class RGBColor {
	r: number;
	g: number;
	b: number;
	constructor() {
		this.r = 255;
		this.g = 255;
		this.b = 255;
	}
}

class ColorPaletteRow {
	color_left: RGBColor;
	color_right: RGBColor;
	row_index: number;

	constructor(row_index: number) {
		this.row_index = row_index;
		this.color_left = new RGBColor;
		this.color_right = new RGBColor;
	}
}


function ColorPalette({ toolbox_globals }: { toolbox_globals: ToolboxGlobals }) {
	if (!toolbox_globals.color_palette)
		toolbox_globals.color_palette = new ColorPaletteGlobals(toolbox_globals, toolbox_globals.multipixel);

	let cp = toolbox_globals.color_palette;

	let rows = new Array<JSX.Element>();

	for (let row of cp.rows) {
		let gradient_count = cp.column_count - 2;
		let gradient_begin = 1;

		let columns = new Array<JSX.Element>();

		for (let i = 0; i < cp.column_count; i++) {
			let class_name = style_toolbox.cell;

			let big = i == 0 || i == cp.column_count - 1;
			let first = i == 0;

			if (!big)
				class_name += " " + style_toolbox.cell_small;

			if (cp.selected_row == row.row_index && cp.selected_column == i)
				class_name += " " + style_toolbox.cell_selected;

			let cell_style: any = {};

			let click_callback: () => void;

			if (big) {
				let color_str;
				if (first) color_str = rgb2hex(row.color_left.r, row.color_left.g, row.color_left.b);
				else color_str = rgb2hex(row.color_right.r, row.color_right.g, row.color_right.b);

				cell_style.backgroundColor = color_str;

				let mod = first ? row.color_left : row.color_right;

				click_callback = () => {
					if (cp.selected_column == i && cp.selected_row == row.row_index) {
						//Run color selector
						if (!toolbox_globals.picker)
							toolbox_globals.picker = new Picker({
								parent: document.getElementById("root")!,
								popup: "bottom",
								alpha: false,
								editor: true
							});

						let picker = toolbox_globals.picker;

						picker.movePopup({ parent: document.getElementById("root")! }, true);

						picker.setColor(rgb2hex(mod.r, mod.g, mod.b), true);

						picker.onChange = (color) => {
							mod.r = color.rgba[0];
							mod.g = color.rgba[1];
							mod.b = color.rgba[2];
							cp.setSelected(row.row_index, i, mod);
							cp.refreshList();
						};

						picker.onClose = () => {
							picker.destroy();
							delete toolbox_globals.picker;
							toolbox_globals.picker = undefined;
						}
					}
					else {
						//Select color
						cp.setSelected(row.row_index, i, mod);
					}
				};
			}
			else {
				//Gradient
				let weight = (i - gradient_begin + 1) / (gradient_count + 1);
				let clr = new RGBColor();
				clr.r = lerp(weight, row.color_left.r, row.color_right.r);
				clr.g = lerp(weight, row.color_left.g, row.color_right.g);
				clr.b = lerp(weight, row.color_left.b, row.color_right.b);
				cell_style.backgroundColor = rgb2hex(clr.r, clr.g, clr.b);

				click_callback = () => {
					cp.setSelected(row.row_index, i, clr);
				};
			}

			columns.push(<div className={class_name} style={cell_style} onClick={click_callback} key={i}>
			</div>);
		}

		rows.push(<div className={style_toolbox.row} key={row.row_index}>
			{columns}
		</div>);
	}

	return <div className={style_toolbox.color_palette}>
		<div>
			{rows}
		</div>
		<div className={style_toolbox.cs_buttons_pair}>
			<div className={style_toolbox.cs_buttons}>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setColumnCount(cp.column_count + 1);
				}}>
					<Icon path="public/img/tool/plus.svg" />
				</div>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setColumnCount(cp.column_count - 1);
				}}>
					<Icon path="public/img/tool/minus.svg" />
				</div>
			</div>
			<div className={style_toolbox.cs_buttons}>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setRowCount(cp.row_count + 1);
				}}>
					<Icon path="public/img/tool/plus.svg" />
				</div>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setRowCount(cp.row_count - 1);
				}}>
					<Icon path="public/img/tool/minus.svg" />
				</div>
			</div>
		</div>
	</div>;
}

function ToolSlider({ name, min, max, initial, onChange }: { name: string, min: number, max: number, initial: number, onChange: (val: number) => void }) {
	return <div className={style_toolbox.slider_container}>
		<input
			type="range"
			min={min}
			max={max}
			defaultValue={initial}
			className={style_toolbox.slider}
			onChange={(e) => {
				onChange(parseInt(e.target.value));
			}}
		/>
		<span className={style_toolbox.slider_title}>{name}</span>
	</div>
}

function ToolSize({ globals, max }: { globals: ToolboxGlobals, max: number }) {
	return <ToolSlider name={"Size"} min={1} max={max} initial={globals.param_tool_size} onChange={(val) => {
		globals.setToolSize(val);
		globals.multipixel.getCursor().tool_size = val;
		globals.multipixel.client.socketSendToolSize(val);
	}} />
}

function ToolSmoothing({ globals }: { globals: ToolboxGlobals }) {
	return <ToolSlider name={"Smoothing"} min={0} max={100} initial={globals.param_tool_smoothing * 100.0} onChange={(val) => {
		globals.setToolSmoothing(val / 100.0);
	}} />
}

function ToolFlow({ globals }: { globals: ToolboxGlobals }) {
	return <ToolSlider name={"Flow"} min={0} max={100} initial={globals.param_tool_flow * 100.0} onChange={(val) => {
		globals.setToolFlow(val / 100.0);
		globals.multipixel.client.socketSendToolFlow(val / 100.0);
	}} />
}

function ToolList({ children }: { children: JSX.Element[] }) {
	return <div className={style_toolbox.tool_settings_parent}>
		{children}
	</div>
}

export function ToolPanel({ globals }: { globals: ToolboxGlobals }) {
	const [tool_type, setToolType] = useState<ToolType>(ToolType.none);
	const [tool_size, setToolSize] = useState(1);
	const [tool_smoothing, setToolSmoothing] = useState(0.0);
	const [tool_flow, setToolFlow] = useState(0.1);
	const [key_palette, setKeyPalette] = useState(0);

	globals.tool_type = tool_type;
	globals.setToolType = setToolType;

	globals.param_tool_size = tool_size;
	globals.setToolSize = setToolSize;

	globals.param_tool_smoothing = tool_smoothing;
	globals.setToolSmoothing = setToolSmoothing;

	globals.param_tool_flow = tool_flow;
	globals.setToolFlow = setToolFlow;

	globals.key_palette = key_palette;
	globals.setKeyPalette = setKeyPalette;

	let tool_settings = undefined;

	if (tool_type == ToolType.none) {
		return <></>;
	}

	if (tool_type == ToolType.brush) {
		tool_settings = <ToolList>
			<ToolSize max={16} globals={globals} />
			<ToolSmoothing globals={globals} />
		</ToolList>
	}
	else if (tool_type == ToolType.spray) {
		tool_settings = <ToolList>
			<ToolSize max={32} globals={globals} />
			<ToolFlow globals={globals} />
			<ToolSmoothing globals={globals} />
		</ToolList>
	}
	else if (tool_type == ToolType.blur) {
		tool_settings = <ToolList>
			<ToolSize max={16} globals={globals} />
			<ToolFlow globals={globals} />
			<ToolSmoothing globals={globals} />
		</ToolList>
	}

	return <div className={style_room.tool_panel}>
		<ColorPalette toolbox_globals={globals} key={key_palette} />
		{tool_settings}
	</div>
}

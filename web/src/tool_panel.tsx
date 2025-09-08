import { useState, type ReactNode } from "react";
import { Icon, Slider } from "./gui_custom";
import style_toolbox from "./toolbox.module.scss"
import style_room from "./views/canvas/room_screen.module.scss";
import { Multipixel, rgb2hex } from "./multipixel"
import Picker from "vanilla-picker";
import { color } from "./color";

export enum ToolType {
	none,
	brush,
	square_brush,
	smooth_brush,
	eraser,
	spray,
	floodfill,
	blur,
	smudge,
	line,
}

interface ColorPaletteState {
	column_count: number;
	row_count: number;
	rows: Array<ColorPaletteRow>;
	selected_row: number;
	selected_column: number;
}

export class ColorPaletteGlobals {
	multipixel!: Multipixel;
	toolbox_globals!: ToolboxGlobals;

	state: ColorPaletteState;

	saveState() {
		const json = JSON.stringify(this.state);
		localStorage.setItem("color_selector", json);
	}

	loadState(): boolean {
		const json = localStorage.getItem("color_selector");
		if (json === null) {
			return false;
		}

		this.state = JSON.parse(json);
		return true;
	}

	constructor(toolbox_globals: ToolboxGlobals, multipixel: Multipixel) {
		this.toolbox_globals = toolbox_globals;
		this.multipixel = multipixel;

		this.state = {
			column_count: 10,
			row_count: 4,
			rows: [],
			selected_column: 0,
			selected_row: 0
		};



		if (!this.loadState()) {
			this.setRowCount(4);
			this.setColumnCount(10);

			// default colors
			this.state.rows[0].color_left = { r: 255, g: 0, b: 0 };
			this.state.rows[0].color_right = { r: 0, g: 255, b: 0 };

			this.state.rows[1].color_left = { r: 0, g: 0, b: 255 };
			this.state.rows[1].color_right = { r: 0, g: 255, b: 0 };

			this.state.rows[2].color_left = { r: 255, g: 0, b: 255 };
			this.state.rows[2].color_right = { r: 0, g: 255, b: 0 };

			this.state.rows[3].color_left = { r: 0, g: 0, b: 0 };
			this.state.rows[3].color_right = { r: 255, g: 255, b: 255 };
		}

		const cur_row = this.state.rows[this.state.selected_row];
		this.setSelectedAndSend(this.state.selected_row, this.state.selected_column, this.state.selected_column < this.state.column_count ? cur_row.color_left : cur_row.color_right);
		this.refreshList();
	}

	setSelectedAndSend(row: number, column: number, color: color.Rgb) {
		this.state.selected_row = row;
		this.state.selected_column = column;
		const instance = this.multipixel.room_instance;
		if (instance.state) {
			instance.state.client.socketSendBrushColor(color.r, color.g, color.b);
		}
		this.refreshList();
		this.saveState();
	}

	setColumnCount(count: number) {
		if (count < 2) count = 2;
		if (count > 30) count = 30;
		this.state.column_count = count;
		this.refreshList();
		this.saveState();
	}

	setRowCount(count: number) {
		if (count < 1) count = 1;
		if (count > 20) count = 20;
		this.state.row_count = count;
		this.refreshList();
		this.saveState();
	}

	refreshList() {
		const state = this.state;
		while (state.rows.length < state.row_count) {
			state.rows.push(new ColorPaletteRow(state.rows.length));
		}

		while (state.rows.length > state.row_count) {
			state.rows.pop();
		}

		this.toolbox_globals.setKeyPalette(this.toolbox_globals.key_palette + 1);
	}

	setColor(color: color.Rgb) {
		const state = this.state;

		let first = true;
		let row = state.selected_row;
		let column = 0;

		if (state.selected_column > state.column_count / 2) {
			column = state.column_count - 1;
			first = false;
		}

		if (first) {
			state.rows[row].color_left = color;
		}
		else {
			state.rows[row].color_right = color;
		}
		this.setSelectedAndSend(row, column, color);

		this.saveState();
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


class ColorPaletteRow {
	color_left: color.Rgb;
	color_right: color.Rgb;
	row_index: number;

	constructor(row_index: number) {
		this.row_index = row_index;
		this.color_left = color.getWhite();
		this.color_right = color.getWhite();
	}
}


function ColorPalette({ toolbox_globals }: { toolbox_globals: ToolboxGlobals }) {
	if (!toolbox_globals.color_palette)
		toolbox_globals.color_palette = new ColorPaletteGlobals(toolbox_globals, toolbox_globals.multipixel);

	let cp = toolbox_globals.color_palette;

	let rows = new Array<ReactNode>();

	const state = cp.state;

	for (let row of state.rows) {
		let gradient_count = state.column_count - 2;
		let gradient_begin = 1;

		let columns = new Array<ReactNode>();

		for (let i = 0; i < state.column_count; i++) {
			let class_name = style_toolbox.cell;

			let is_first_or_last = i == 0 || i == state.column_count - 1;
			let is_first = i == 0;

			if (!is_first_or_last) {
				class_name += " " + style_toolbox.cell_small;
			}

			if (state.selected_row == row.row_index && state.selected_column == i) {
				class_name += " " + style_toolbox.cell_selected;
			}

			let cell_style: any = {};

			let to_modify = i < state.column_count / 2 ? row.color_left : row.color_right;
			const click_callback = () => {
				if (state.selected_column == i && state.selected_row == row.row_index) {
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

					picker.setColor(rgb2hex(to_modify.r, to_modify.g, to_modify.b), true);

					picker.onChange = (color) => {
						to_modify.r = color.rgba[0];
						to_modify.g = color.rgba[1];
						to_modify.b = color.rgba[2];
						cp.setSelectedAndSend(row.row_index, i, to_modify);
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
					if (is_first_or_last) {
						cp.setSelectedAndSend(row.row_index, i, to_modify);
					}
					else {
						//Gradient
						let weight = (i - gradient_begin + 1) / (gradient_count + 1);
						let clr = color.lerpSrgbInLab(weight, row.color_left, row.color_right);
						cp.setSelectedAndSend(row.row_index, i, clr);
					}
				}
			};

			if (is_first_or_last) {
				let color_str;
				if (is_first) color_str = rgb2hex(row.color_left.r, row.color_left.g, row.color_left.b);
				else color_str = rgb2hex(row.color_right.r, row.color_right.g, row.color_right.b);
				cell_style.backgroundColor = color_str;
			}
			else {
				//Gradient
				let weight = (i - gradient_begin + 1) / (gradient_count + 1);
				let clr = color.lerpSrgbInLab(weight, row.color_left, row.color_right);
				cell_style.backgroundColor = rgb2hex(clr.r, clr.g, clr.b);
			}

			columns.push(<div className={class_name} style={cell_style} onClick={click_callback} key={i}>
			</div>);
		}

		rows.push(<div className={style_toolbox.row} key={row.row_index}>
			{columns}
		</div>);
	}

	return <div className={style_toolbox.color_palette}>
		<div className={style_toolbox.cs_rows}>
			{rows}
		</div>
		<div className={style_toolbox.cs_buttons_pair}>
			<div className={style_toolbox.cs_buttons}>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setColumnCount(state.column_count + 1);
				}}>
					<Icon path="img/tool/plus.svg" />
				</div>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setColumnCount(state.column_count - 1);
				}}>
					<Icon path="img/tool/minus.svg" />
				</div>
			</div>
			<div className={style_toolbox.cs_buttons}>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setRowCount(state.row_count + 1);
				}}>
					<Icon path="img/tool/plus.svg" />
				</div>
				<div className={style_toolbox.cs_button} onClick={() => {
					cp.setRowCount(state.row_count - 1);
				}}>
					<Icon path="img/tool/minus.svg" />
				</div>
			</div>
		</div>
	</div>;
}

function ToolSlider({ name, min, max, steps, initial, onChange }: { name: string, min: number, max: number, steps?: number, initial: number, onChange: (val: number) => void }) {
	const [value, setValue] = useState(initial);

	return <Slider title={name} mapped_value={value} setMappedValue={setValue} steps={steps} min={min} max={max} width={200} on_change={(num) => {
		onChange(num);
	}} />
}

function ToolSize({ globals, max }: { globals: ToolboxGlobals, max: number }) {
	return <ToolSlider name={"Size"} min={1} max={max} steps={max} initial={globals.param_tool_size} onChange={(val) => {
		globals.setToolSize(val);
		const instance = globals.multipixel.room_instance;
		if (instance) {
			instance.cursor.tool_size = val;
			if (instance.state) {
				instance.state.client.socketSendToolSize(val);
			}
		}
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
		const instance = globals.multipixel.room_instance;
		if (instance && instance.state) {
			instance.state.client.socketSendToolFlow(val / 100.0);
		}
	}} />
}

function ToolList({ children }: { children: ReactNode }) {
	return <div className={style_toolbox.tool_settings_parent}>
		{children}
	</div>
}

export function ToolPanel({ globals }: { globals: ToolboxGlobals }) {
	const [tool_type, setToolType] = useState<ToolType>(ToolType.none);
	const [tool_size, setToolSize] = useState(1);
	const [tool_smoothing, setToolSmoothing] = useState(0.0);
	const [tool_flow, setToolFlow] = useState(0.5);
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
	let color_palette = undefined;

	if (tool_type == ToolType.none) {
		return <></>;
	}

	if (tool_type != ToolType.eraser) {
		color_palette = <ColorPalette toolbox_globals={globals} key={key_palette} />
	}

	switch (tool_type) {
		case ToolType.brush:
		case ToolType.square_brush:
		case ToolType.eraser: {
			tool_settings = <ToolList>
				<ToolSize max={32} globals={globals} />
				<ToolSmoothing globals={globals} />
			</ToolList>
			break;
		}
		case ToolType.line: {
			tool_settings = <ToolList>
				<ToolSize max={32} globals={globals} />
			</ToolList>;
			break;
		}
		case ToolType.spray:
		case ToolType.blur:
		case ToolType.smudge:
		case ToolType.smooth_brush: {
			let size = 32;
			switch (tool_type) {
				case ToolType.spray: {
					size = 48;
					break;
				}
				case ToolType.smooth_brush: {
					size = 48;
					break;
				}
			}

			tool_settings = <ToolList>
				<ToolSize max={size} globals={globals} />
				<ToolFlow globals={globals} />
				<ToolSmoothing globals={globals} />
			</ToolList>
			break;
		}
	}


	return <div className={style_room.tool_panel}>
		{color_palette}
		{tool_settings}
	</div>
}

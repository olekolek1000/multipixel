import { Multipixel, rgb2hex } from "./multipixel"
import { lerp } from "./timestep";
import Picker from "vanilla-picker";


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

class Row {
	row!: HTMLElement;
	color_palette: ColorPalette;
	color_left: RGBColor;
	color_right: RGBColor;
	row_index: number;

	constructor(row_index: number, color_palette: ColorPalette) {
		this.row_index = row_index;
		this.color_palette = color_palette;
		this.color_left = new RGBColor;
		this.color_right = new RGBColor;
	}

	initGUI(parent: HTMLElement, column_count: number) {
		this.row = document.createElement("div");
		this.row.classList.add("mpc_row");
		parent.appendChild(this.row);

		let gradient_count = column_count - 2;
		let gradient_begin = 1;

		for (let i = 0; i < column_count; i++) {
			let cell = document.createElement("div");

			let big = i == 0 || i == column_count - 1;
			let first = i == 0;

			cell.classList.add("mpc_cell");
			if (!big)
				cell.classList.add("mpc_cell_small");

			if (this.color_palette.selected_row == this.row_index && this.color_palette.selected_column == i)
				cell.classList.add("mpc_cell_selected");

			if (big) {
				let color_str;
				if (first) color_str = rgb2hex(this.color_left.r, this.color_left.g, this.color_left.b);
				else color_str = rgb2hex(this.color_right.r, this.color_right.g, this.color_right.b);

				cell.style.backgroundColor = color_str;

				let mod = first ? this.color_left : this.color_right;

				cell.addEventListener("click", () => {
					if (this.color_palette.selected_column == i && this.color_palette.selected_row == this.row_index) {
						//Run color selector
						let parent = document.getElementById("mp_top_panel") as HTMLElement;
						let picker = new Picker({ parent: parent });
						picker.setColor(rgb2hex(mod.r, mod.g, mod.b), true);
						picker.setOptions({ alpha: false, popup: "bottom", editor: true });

						this.color_palette.multipixel.setEventsEnabled(false);

						picker.onChange = (color) => {
							mod.r = color.rgba[0];
							mod.g = color.rgba[1];
							mod.b = color.rgba[2];
							this.color_palette.setSelected(this.row_index, i, mod);
							this.color_palette.refreshList();
						};

						picker.onClose = () => {
							picker.destroy()
							this.color_palette.multipixel.setEventsEnabled(true);
						}
					}
					else {
						//Select color
						this.color_palette.setSelected(this.row_index, i, mod);
					}
				});
			}
			else {
				//Gradient
				let weight = (i - gradient_begin + 1) / (gradient_count + 1);
				let clr = new RGBColor();
				clr.r = lerp(weight, this.color_left.r, this.color_right.r);
				clr.g = lerp(weight, this.color_left.g, this.color_right.g);
				clr.b = lerp(weight, this.color_left.b, this.color_right.b);
				cell.style.backgroundColor = rgb2hex(clr.r, clr.g, clr.b);

				cell.addEventListener("click", () => {
					this.color_palette.setSelected(this.row_index, i, clr);
				});
			}

			this.row.appendChild(cell);
		}

	}
}

export class ColorPalette {
	parent: HTMLElement;
	multipixel: Multipixel;

	rows: Array<Row>;
	column_count: number = 1;
	row_count: number = 1;

	selected_row: number = 0;
	selected_column: number = 0;

	constructor(multipixel: Multipixel, parent: HTMLElement) {
		this.parent = parent;
		this.multipixel = multipixel;
		this.rows = new Array<Row>();

		this.setColumnCount(10);
		this.setRowCount(4);

		document.getElementById("mpc_resize_col_add")!.addEventListener("click", () => {
			this.setColumnCount(this.column_count + 1);
		});

		document.getElementById("mpc_resize_col_sub")!.addEventListener("click", () => {
			this.setColumnCount(this.column_count - 1);
		});

		document.getElementById("mpc_resize_row_add")!.addEventListener("click", () => {
			this.setRowCount(this.row_count + 1);
		});

		document.getElementById("mpc_resize_row_sub")!.addEventListener("click", () => {
			this.setRowCount(this.row_count - 1);
		});

		let clr = new RGBColor();
		clr.r = 255;
		clr.g = 0;
		clr.b = 0;
		this.rows[0].color_left = clr;
		this.setSelected(0, 0, clr);
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

	setSelected(row: number, column: number, color: RGBColor) {
		this.selected_row = row;
		this.selected_column = column;
		this.multipixel.client.socketSendBrushColor(color.r, color.g, color.b);
		this.refreshList();
	}

	refreshList() {
		//Remove all elements
		while (this.parent.firstChild)
			this.parent.removeChild(this.parent.firstChild);

		while (this.rows.length < this.row_count)
			this.rows.push(new Row(this.rows.length, this));

		while (this.rows.length > this.row_count)
			this.rows.pop();

		this.rows.forEach((row) => {
			row.initGUI(this.parent, this.column_count);
		});
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
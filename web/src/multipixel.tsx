import { Timestep } from "./timestep";
import { globals } from ".";
import { RoomScreenGlobals } from "./views/canvas/room_screen"
import { ToolboxGlobals } from "./tool_panel";
import { RoomInstance } from "./room_instance";


function dec2hex(n: number) {
	n = Math.round(n);
	if (n < 0) n = 0;
	if (n > 255) n = 255;
	return n.toString(16).padStart(2, '0');
}

export function rgb2hex(red: number, green: number, blue: number) {
	return '#' + dec2hex(red) + dec2hex(green) + dec2hex(blue);
}

export interface ConnectParams {
	host: string;
	nickname: string;
	room_name: string;
}
export class Multipixel {
	room_instance: RoomInstance;
	timestep?: Timestep;
	toolbox_globals!: ToolboxGlobals;
	room_screen_globals!: RoomScreenGlobals;

	constructor(params: {
		connect_params: ConnectParams,
		connection_callback: (error_str?: string) => void,
	}) {
		this.toolbox_globals = new ToolboxGlobals(this);
		this.room_screen_globals = new RoomScreenGlobals();
		this.timestep = new Timestep(60.0);

		this.room_instance = new RoomInstance({
			connect_params: params.connect_params,
			room_screen_globals: this.room_screen_globals,
			toolbox_globals: this.toolbox_globals,
			connection_callback: params.connection_callback,
		})


		// Start render loop
		this.draw();

		this.initEssential();
	}

	draw() {
		while (this.timestep && this.timestep.onTick()) {
			if (this.room_instance) {
				this.room_instance.tick();
			}
		}

		if (this.room_instance) {
			this.room_instance.draw();
		}

		window.requestAnimationFrame(() => {
			this.draw();
		});
	}

	initEssential() {
		setInterval(() => {
			if (this.room_instance) {
				this.room_instance.updateBoundary();
			}
		}, 200);

		setInterval(() => {
			if (this.room_instance && this.room_instance.state) {
				this.room_instance.state.client.socketSendPing();
			}
		}, 15000);

		document.addEventListener('keydown', (event) => {
			if (event.ctrlKey && event.key === 'z') {
				const instance = this.room_instance;
				if (instance && instance.state) {
					instance.state.client.socketSendUndo();
				}
			}
		});

		window.addEventListener("blur", (e) => {
			if (this.room_instance) {
				this.room_instance.handleBlurEvent(e);
			}
		});

		globals.root.addEventListener("contextmenu", (e) => {
			e.preventDefault();
			return false;
		});
	}
}

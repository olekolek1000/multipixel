import React, { useEffect, useState } from "react";
import { Multipixel } from "./multipixel";
import { ToolPanel, ToolType } from "./tool_panel"
import style_room from "./room_screen.scss"
import { BoxDown, BoxRight, ButtonTool, Icon, Tooltip } from "./gui_custom";
import { ChatRender } from "./chat";

export class RoomRefs {
	canvas_render!: HTMLElement;
};

export class RoomScreenGlobals {
	processing_status_text?: string;
	setProcessingStatusText?: any;

	setMousePosText?: any;
}

export function RoomScreen({ globals, multipixel, refs_callback }: { globals: RoomScreenGlobals, multipixel: Multipixel, refs_callback: (refs: RoomRefs) => void }) {
	const canvas_render = React.useRef(null);
	const [player_list, setPlayerList] = useState<JSX.Element>();
	const [processing_status_text, setProcessingStatusText] = useState("");
	const [tool_type, setToolType] = useState<ToolType>(multipixel.toolbox_globals.tool_type);
	const [mouse_pos_text, setMousePosText] = useState<JSX.Element>(<></>);

	globals.processing_status_text = processing_status_text;
	globals.setProcessingStatusText = setProcessingStatusText;

	globals.setMousePosText = setMousePosText;

	useEffect(() => {
		multipixel.toolbox_globals.setToolType(tool_type);
	}, [tool_type]);

	multipixel.callback_player_update = () => {
		let list = multipixel.getPlayerList();

		const generateList = () => {
			let arr = new Array<JSX.Element>();

			for (let name of list) {
				arr.push(<div key={name}>
					{name}
				</div>)
			}

			return arr;
		}

		setPlayerList(<>
			<Tooltip title={
				<React.Fragment>
					{generateList()}
				</React.Fragment>
			}>
				<Icon path="public/img/tool/user.svg" />
			</Tooltip>
			<span className={style_room.users_online}>{list.length} {list.length == 1 ? "user" : "users"}</span>
		</>);
	}

	useEffect(() => {
		refs_callback({
			canvas_render: canvas_render.current!
		})
	}, []);


	return <div id="mp_screen">
		<canvas id="canvas_render" ref={canvas_render} width="100%" height="100%"></canvas>
		<ToolPanel toolbox_globals={multipixel.toolbox_globals} />
		<ChatRender chat={multipixel.chat} />
		<div className={style_room.toolboxes}>
			<div className={style_room.toolbox}>
				<Tooltip title="Brush">
					<ButtonTool highlighted={tool_type == ToolType.brush} on_click={() => {
						multipixel.handleButtonToolBrush();
						setToolType(ToolType.brush);
					}}>
						<Icon path="public/img/tool/brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Floodfill">
					<ButtonTool highlighted={tool_type == ToolType.floodfill} on_click={() => {
						multipixel.handleButtonToolFloodfill();
						setToolType(ToolType.floodfill);
					}}>
						<Icon path="public/img/tool/floodfill.svg" />
					</ButtonTool>
				</Tooltip>
			</div>
			<div className={style_room.toolbox}>
				<Tooltip title="Undo">
					<ButtonTool on_click={() => {
						multipixel.handleButtonUndo();
					}}>
						<Icon path="public/img/tool/undo.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Reset zoom to 100%">
					<ButtonTool on_click={() => {
						multipixel.handleButtomZoom1_1();
					}}>
						<Icon path="public/img/tool/100.svg" />
					</ButtonTool>
				</Tooltip>
				{processing_status_text}
				{player_list}
				{mouse_pos_text}
			</div>
		</div>
	</div>
}
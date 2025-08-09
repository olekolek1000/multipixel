import React, { type ReactNode, useEffect, useState } from "react";
import { ToolPanel, ToolType } from "../../tool_panel"
import style_room from "./room_screen.module.scss"
import { BoxRight, ButtonTool, Icon, Tooltip } from "../../gui_custom";
import { ChatRender } from "../../chat/chat";
import tool from "../../tool";
import type { RoomInstance } from "@/room_instance";


export class RoomScreenGlobals {
	processing_status_text?: string;
	setProcessingStatusText?: any;

	setMousePosText?: any;
}

export interface RoomScreenRefs {
	canvas_render: HTMLCanvasElement;
}

export function RoomScreen({ globals, instance, refs_callback }: { globals: RoomScreenGlobals, instance: RoomInstance, refs_callback: (refs: RoomScreenRefs) => void }) {
	const canvas_render = React.useRef(null);
	const [player_list, setPlayerList] = useState<ReactNode>();
	const [processing_status_text, setProcessingStatusText] = useState("");
	const [tool_type, setToolType] = useState<ToolType>(instance.toolbox_globals.tool_type);
	const [mouse_pos_text, setMousePosText] = useState<ReactNode>(<></>);

	const updateTool = (tool_id: tool.ToolID, tool_type: ToolType) => {
		instance.selectTool(tool_id);
		setToolType(tool_type);
	}

	globals.processing_status_text = processing_status_text;
	globals.setProcessingStatusText = setProcessingStatusText;

	globals.setMousePosText = setMousePosText;

	useEffect(() => {
		instance.toolbox_globals.setToolType(tool_type);
	}, [tool_type]);

	instance.callback_user_update = () => {
		let list = instance.getUserList();

		const generateList = () => {
			let arr = new Array<ReactNode>();

			for (let name of list) {
				arr.push(<span key={name}>
					{name}
				</span>)
			}

			return arr;
		}

		setPlayerList(<>
			<Tooltip title={
				<BoxRight nowrap>{generateList()}</BoxRight>
			}>
				<Icon path="img/tool/user.svg" />
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
		<ToolPanel globals={instance.toolbox_globals} />
		{instance.state && <ChatRender chat={instance.state.chat} />}
		<div className={style_room.toolboxes}>
			<div className={style_room.toolbox}>
				<Tooltip title="Brush">
					<ButtonTool highlighted={tool_type == ToolType.brush} on_click={() => {
						updateTool(tool.ToolID.Brush, ToolType.brush);
					}}>
						<Icon path="img/tool/brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Square brush">
					<ButtonTool highlighted={tool_type == ToolType.square_brush} on_click={() => {
						updateTool(tool.ToolID.SquareBrush, ToolType.square_brush);
					}}>
						<Icon path="img/tool/square_brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Smooth brush">
					<ButtonTool highlighted={tool_type == ToolType.smooth_brush} on_click={() => {
						updateTool(tool.ToolID.SmoothBrush, ToolType.smooth_brush);
					}}>
						<Icon path="img/tool/smooth_brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Line">
					<ButtonTool highlighted={tool_type == ToolType.line} on_click={() => {
						updateTool(tool.ToolID.Line, ToolType.line);
					}}>
						<Icon path="img/tool/line.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Floodfill">
					<ButtonTool highlighted={tool_type == ToolType.floodfill} on_click={() => {
						updateTool(tool.ToolID.Floodfill, ToolType.floodfill);
					}}>
						<Icon path="img/tool/floodfill.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Spray">
					<ButtonTool highlighted={tool_type == ToolType.spray} on_click={() => {
						updateTool(tool.ToolID.Spray, ToolType.spray);
					}}>
						<Icon path="img/tool/spray.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Blur">
					<ButtonTool highlighted={tool_type == ToolType.blur} on_click={() => {
						updateTool(tool.ToolID.Blur, ToolType.blur);
					}}>
						<Icon path="img/tool/blur.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Smudge">
					<ButtonTool highlighted={tool_type == ToolType.smudge} on_click={() => {
						updateTool(tool.ToolID.Smudge, ToolType.smudge);
					}}>
						<Icon path="img/tool/smudge.svg" />
					</ButtonTool>
				</Tooltip>
			</div>
			<div className={style_room.toolbox}>
				<Tooltip title="Undo">
					<ButtonTool on_click={() => {
						instance.actionUndo();
					}}>
						<Icon path="img/tool/undo.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Reset zoom to 100%">
					<ButtonTool on_click={() => {
						instance.actionZoom1_1();
					}}>
						<Icon path="img/tool/100.svg" />
					</ButtonTool>
				</Tooltip>
				{processing_status_text}
				{player_list}
				{mouse_pos_text}
			</div>
		</div>
	</div>
}
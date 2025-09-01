import { type ReactNode, useEffect, useRef, useState } from "react";
import { ToolPanel, ToolType } from "../../tool_panel"
import style_room from "./room_screen.module.scss"
import { BoxRight, ButtonTool, Icon, Tooltip } from "../../gui_custom";
import { ChatRender } from "../../chat/chat";
import tool from "../../tool";
import type { RoomInstance } from "@/room_instance";

export class RoomScreenGlobals {
	processing_status_text?: string;
	setProcessingStatusText?: any;

	setTextCursorPosition?: any;
}

export interface RoomScreenRefs {
	canvas_render: HTMLCanvasElement;
}


export function RoomScreen({ globals, instance, refs_callback }: { globals: RoomScreenGlobals, instance: RoomInstance, refs_callback: (refs: RoomScreenRefs) => void }) {
	const canvas_render = useRef<HTMLCanvasElement>(null);
	const [player_list, setPlayerList] = useState<ReactNode>();
	const [processing_status_text, setProcessingStatusText] = useState("");
	const [cur_tool_type, setCurrentToolType] = useState<ToolType>(instance.toolbox_globals.tool_type);
	const [text_cursor_position, setTextCursorPosition] = useState<ReactNode>(<></>);

	const updateTool = (tool_id: tool.ToolID, tool_type: ToolType) => {
		instance.selectTool(tool_id);
		setCurrentToolType(tool_type);
	}

	const ToolCell = ({ display_name, tool_type, tool_id, svg_path }: { display_name: string, tool_type: ToolType, tool_id: tool.ToolID, svg_path: string }) => {
		return <Tooltip title={display_name}>
			<ButtonTool highlighted={cur_tool_type == tool_type} on_click={() => {
				updateTool(tool_id, tool_type);
			}}>
				<Icon path={svg_path} />
			</ButtonTool>
		</Tooltip>
	}

	globals.processing_status_text = processing_status_text;
	globals.setProcessingStatusText = setProcessingStatusText;

	globals.setTextCursorPosition = setTextCursorPosition;

	useEffect(() => {
		instance.toolbox_globals.setToolType(cur_tool_type);
	}, [cur_tool_type]);

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
				<ToolCell display_name="Brush" tool_type={ToolType.brush} tool_id={tool.ToolID.Brush} svg_path="img/tool/brush.svg" />
				<ToolCell display_name="Square brush" tool_type={ToolType.square_brush} tool_id={tool.ToolID.SquareBrush} svg_path="img/tool/square_brush.svg" />
				<ToolCell display_name="Smooth brush" tool_type={ToolType.smooth_brush} tool_id={tool.ToolID.SmoothBrush} svg_path="img/tool/smooth_brush.svg" />
				<ToolCell display_name="Eraser" tool_type={ToolType.eraser} tool_id={tool.ToolID.Eraser} svg_path="img/tool/eraser.svg" />
				<ToolCell display_name="Line" tool_type={ToolType.line} tool_id={tool.ToolID.Line} svg_path="img/tool/line.svg" />
				<ToolCell display_name="Floodfill" tool_type={ToolType.floodfill} tool_id={tool.ToolID.Floodfill} svg_path="img/tool/floodfill.svg" />
				<ToolCell display_name="Spray" tool_type={ToolType.spray} tool_id={tool.ToolID.Spray} svg_path="img/tool/spray.svg" />
				<ToolCell display_name="Blur" tool_type={ToolType.blur} tool_id={tool.ToolID.Blur} svg_path="img/tool/blur.svg" />
				<ToolCell display_name="Smudge" tool_type={ToolType.smudge} tool_id={tool.ToolID.Smudge} svg_path="img/tool/smudge.svg" />
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
				{player_list}
			</div>
		</div>
		<div className={style_room.bottom_text}>
			{processing_status_text}
			{text_cursor_position}
		</div>
	</div>
}
import React, { useEffect, useState } from "react";
import { Multipixel } from "./multipixel";
import { ToolPanel, ToolType } from "./tool_panel"
import style_room from "./room_screen.module.scss"
import { BoxRight, ButtonTool, Icon, Tooltip } from "./gui_custom";
import { ChatRender } from "./chat";
import tool from "./tool";
import { JSX } from "react/jsx-runtime";

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
		<ToolPanel globals={multipixel.toolbox_globals} />
		<ChatRender chat={multipixel.chat} />
		<div className={style_room.toolboxes}>
			<div className={style_room.toolbox}>
				<Tooltip title="Brush">
					<ButtonTool highlighted={tool_type == ToolType.brush} on_click={() => {
						multipixel.selectTool(tool.ToolID.Brush);
						setToolType(ToolType.brush);
					}}>
						<Icon path="img/tool/brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Square brush">
					<ButtonTool highlighted={tool_type == ToolType.square_brush} on_click={() => {
						multipixel.selectTool(tool.ToolID.SquareBrush);
						setToolType(ToolType.square_brush);
					}}>
						<Icon path="img/tool/square_brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Smooth brush">
					<ButtonTool highlighted={tool_type == ToolType.smooth_brush} on_click={() => {
						multipixel.selectTool(tool.ToolID.SmoothBrush);
						setToolType(ToolType.smooth_brush);
					}}>
						<Icon path="img/tool/smooth_brush.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Floodfill">
					<ButtonTool highlighted={tool_type == ToolType.floodfill} on_click={() => {
						multipixel.selectTool(tool.ToolID.Floodfill);
						setToolType(ToolType.floodfill);
					}}>
						<Icon path="img/tool/floodfill.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Spray">
					<ButtonTool highlighted={tool_type == ToolType.spray} on_click={() => {
						multipixel.selectTool(tool.ToolID.Spray);
						setToolType(ToolType.spray);
					}}>
						<Icon path="img/tool/spray.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Blur">
					<ButtonTool highlighted={tool_type == ToolType.blur} on_click={() => {
						multipixel.selectTool(tool.ToolID.Blur);
						setToolType(ToolType.blur);
					}}>
						<Icon path="img/tool/blur.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Smudge">
					<ButtonTool highlighted={tool_type == ToolType.smudge} on_click={() => {
						multipixel.selectTool(tool.ToolID.Smudge);
						setToolType(ToolType.smudge);
					}}>
						<Icon path="img/tool/smudge.svg" />
					</ButtonTool>
				</Tooltip>
			</div>
			<div className={style_room.toolbox}>
				<Tooltip title="Undo">
					<ButtonTool on_click={() => {
						multipixel.handleButtonUndo();
					}}>
						<Icon path="img/tool/undo.svg" />
					</ButtonTool>
				</Tooltip>
				<Tooltip title="Reset zoom to 100%">
					<ButtonTool on_click={() => {
						multipixel.handleButtomZoom1_1();
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
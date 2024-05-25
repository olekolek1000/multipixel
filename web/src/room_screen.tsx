import React, { useEffect, useState } from "react";
import { Multipixel } from "./multipixel";
import { AppBar, Button, Divider, Grid, IconButton, List, ListItemButton, ListItemIcon, Toolbar, Tooltip, TooltipProps, Typography } from "@mui/material";
import { styled } from '@mui/material/styles';
import { ToolType, Toolbox } from "./toolbox"
import style_room from "./room_screen.scss"
import MoneyIcon from '@mui/icons-material/Money';
import UndoIcon from '@mui/icons-material/Undo';
import FormatColorFillIcon from '@mui/icons-material/FormatColorFill';
import BrushIcon from '@mui/icons-material/Brush';
import { BoxRight } from "./gui_custom";
import PersonIcon from '@mui/icons-material/Person';

export class RoomRefs {
  canvas_render!: HTMLElement;
};

export class RoomScreenGlobals {
  processing_status_text?: string;
  setProcessingStatusText?: any;

  mouse_pos_text?: string;
  setMousePosText?: any;
}

export function RoomScreen({ globals, multipixel, refs_callback }: { globals: RoomScreenGlobals, multipixel: Multipixel, refs_callback: (refs: RoomRefs) => void }) {
  const canvas_render = React.useRef(null);
  const [player_list, setPlayerList] = useState<JSX.Element>();
  const [processing_status_text, setProcessingStatusText] = useState("");
  const [tool_type, setToolType] = useState<ToolType>(multipixel.toolbox_globals.tool_type);
  const [mouse_pos_text, setMousePosText] = useState<string>("0,0");

  globals.processing_status_text = processing_status_text;
  globals.setProcessingStatusText = setProcessingStatusText;

  globals.mouse_pos_text = mouse_pos_text;
  globals.setMousePosText = setMousePosText;

  useEffect(() => {
    multipixel.toolbox_globals.setToolType(tool_type);
  }, [tool_type]);

  multipixel.callback_player_update = () => {
    let list = multipixel.getPlayerList();

    const generateList = () => {
      let arr = new Array<JSX.Element>();

      for (let name of list) {
        arr.push(<ListItemButton key={name}>
          <ListItemIcon>
            <PersonIcon />
          </ListItemIcon>
          {name}
        </ListItemButton>)
      }

      return <List>
        {arr}
      </List>
    }

    setPlayerList(<BoxRight>
      {list.length} {list.length == 1 ? "user" : "users"} online
      <Tooltip title={
        <React.Fragment>
          {generateList()}
        </React.Fragment>
      }>
        <IconButton>
          <PersonIcon />
        </IconButton>
      </Tooltip>
    </BoxRight>);
  }

  useEffect(() => {
    refs_callback({
      canvas_render: canvas_render.current!
    })
  }, []);

  const getStyleSel = (type: ToolType) => {
    if (type == tool_type) {
      return {
        backgroundColor: "rgba(255, 255, 255, 0.3)"
      }
    }
    return undefined;
  }

  return <div id="mp_screen">
    <canvas id="canvas_render" ref={canvas_render} width="100%" height="100%"></canvas>
    <div id="mp_chat_box">
      <div id="mp_chat_history"></div>
      <input id="mp_chat_input" type="text" />
    </div>
    <div className={style_room.top_panel}>
      <Grid justifyContent="space-between" direction="row" height={"100%"} container>
        <BoxRight>
          <Tooltip title="Undo">
            <IconButton onClick={() => {
              multipixel.handleButtonUndo();
            }}>
              <UndoIcon />
            </IconButton>
          </Tooltip>
          <Tooltip title="Reset zoom to 100%">
            <IconButton onClick={() => {
              multipixel.handleButtomZoom1_1();
            }}>
              <MoneyIcon />
            </IconButton>
          </Tooltip>
          <Divider orientation="vertical" />
          <Tooltip title="Brush">
            <IconButton style={getStyleSel(ToolType.brush)} onClick={() => {
              multipixel.handleButtonToolBrush();
              setToolType(ToolType.brush);
            }}>
              <BrushIcon />
            </IconButton>
          </Tooltip>
          <Tooltip title="Floodfill">
            <IconButton style={getStyleSel(ToolType.floodfill)} onClick={() => {
              multipixel.handleButtonToolFloodfill();
              setToolType(ToolType.floodfill);
            }}>
              <FormatColorFillIcon />
            </IconButton>
          </Tooltip>
          {processing_status_text}
        </BoxRight>
        <BoxRight>
          {mouse_pos_text}
        </BoxRight>
        {player_list}
      </Grid>
    </div>
    <Toolbox toolbox_globals={multipixel.toolbox_globals} />
  </div>
}
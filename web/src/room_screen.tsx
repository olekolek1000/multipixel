import React, { useEffect, useState } from "react";
import { Multipixel } from "./multipixel";
import { AppBar, Button, Divider, Grid, IconButton, List, ListItemButton, ListItemIcon, Toolbar, Tooltip, TooltipProps, Typography } from "@mui/material";
import { styled } from '@mui/material/styles';

import style_room from "./room_screen.scss"

import MoneyIcon from '@mui/icons-material/Money';
import UndoIcon from '@mui/icons-material/Undo';
import FormatColorFillIcon from '@mui/icons-material/FormatColorFill';
import BrushIcon from '@mui/icons-material/Brush';
import { BoxRight } from "./gui_custom";
import PersonIcon from '@mui/icons-material/Person';

export class RoomRefs {
  canvas_render!: HTMLElement;
  mp_slider_brush_size!: HTMLElement;
  mp_slider_brush_smoothing!: HTMLElement;
  mpc_color_palette!: HTMLElement;
};

export function RoomScreen({ multipixel, refs_callback }: { multipixel: Multipixel, refs_callback: (refs: RoomRefs) => void }) {
  const canvas_render = React.useRef(null);
  const mp_slider_brush_size = React.useRef(null);
  const mp_slider_brush_smoothing = React.useRef(null);
  const button_zoom_1_1 = React.useRef(null);
  const mpc_color_palette = React.useRef(null);

  const [player_list, setPlayerList] = useState<JSX.Element>();

  const [selected_tool_index, setSelectedToolIndex] = useState(0);

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
      canvas_render: canvas_render.current!,
      mp_slider_brush_size: mp_slider_brush_size.current!,
      mp_slider_brush_smoothing: mp_slider_brush_smoothing.current!,
      mpc_color_palette: mpc_color_palette.current!
    })
  }, []);

  const getStyleSel = (index: number) => {
    if (index == selected_tool_index) {
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
      <Grid justifyContent="space-between" direction="row" container>
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
            <IconButton style={getStyleSel(0)} onClick={() => {
              multipixel.handleButtonToolBrush();
              setSelectedToolIndex(0);
            }}>
              <BrushIcon />
            </IconButton>
          </Tooltip>
          <Tooltip title="Floodfill">
            <IconButton style={getStyleSel(1)} onClick={() => {
              multipixel.handleButtonToolFloodfill();
              setSelectedToolIndex(1);
            }}>
              <FormatColorFillIcon />
            </IconButton>
          </Tooltip>
        </BoxRight>
        {player_list}
      </Grid>
    </div>
    <div id="mp_top_panel">
      <div id="mpc_color_palette" ref={mpc_color_palette}></div>
      <div className="mpc_inline">
        <div className="mpc_control" id="mpc_resize_col_add">+</div>
        <br />
        <div className="mpc_control" id="mpc_resize_col_sub">-</div>
      </div>
      <div className="mpc_clear">
        <div className="mpc_control" id="mpc_resize_row_add">+</div>
        <div className="mpc_control" id="mpc_resize_row_sub">-</div>
      </div>
      <div className="mp_slider_container">
        <input
          type="range"
          min="1"
          max="8"
          value="1"
          className="mp_slider"
          id="mp_slider_brush_size"
          ref={mp_slider_brush_size}
        />
        <span className="mp_slider_title"> Size </span>
      </div>
      <div className="mp_slider_container">
        <input
          type="range"
          min="0"
          max="100"
          value="10"
          className="mp_slider"
          id="mp_slider_brush_smoothing"
          ref={mp_slider_brush_smoothing}
        />
        <span className="mp_slider_title"> Smoothing </span>
      </div>
    </div>
  </div>
}
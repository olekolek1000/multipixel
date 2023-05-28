import { Box, Button, Card, Checkbox, FormControl, FormControlLabel, FormHelperText, IconButton, LinearProgress, LinearProgressProps, TextField, Tooltip, Typography } from "@mui/material";
import React, { useState } from "react";

export function CustomTextField(
  { required, label, valfunc, type, error, error_text, onReturnPress }:
    {
      required?: boolean,
      label: string,
      valfunc: [value: string, func: (str: string) => void],
      type?: React.HTMLInputTypeAttribute,
      error?: boolean,
      error_text?: string,
      onReturnPress?: () => void
    }) {

  return <TextField
    required={required}
    label={label}
    variant="filled"
    value={valfunc[0]}
    type={type}
    error={error}
    fullWidth
    helperText={error ? error_text : undefined}
    onChange={e => valfunc[1](e.target.value)}
    onKeyDown={
      (e) => {
        if (e.key == "Enter" && onReturnPress)
          onReturnPress();
      }
    } />
}

//Inline box, dir: right
export function BoxRight(
  { children, nowrap, ref }: { children: any, nowrap?: boolean, ref?: any }
) {
  return <Box sx={{
    display: "flex",
    gap: "0.25cm",
    flexWrap: nowrap ? undefined : "wrap",
    alignItems: "center",
  }} ref={ref}>
    {children}
  </Box>
}

//Inline box, dir: down
export function BoxDown(
  { children, nogap, nowrap, center_horiz, center_vert }: { children: any, nogap?: boolean, nowrap?: boolean, center_horiz?: boolean, center_vert?: boolean }
) {
  return <Box sx={{
    display: "flex",
    gap: nogap ? undefined : "0.25cm",
    flexWrap: nowrap ? undefined : "wrap",
    flexDirection: "column",
    alignItems: center_horiz ? "center" : undefined,
    justifyContent: center_vert ? "center" : undefined
  }} >
    {children}
  </Box>
}

export function FormControlSpaced({ children, center_horiz }: { children: any, center_horiz?: boolean }) {
  return <FormControl>
    <BoxDown center_horiz={center_horiz}>
      {children}
    </BoxDown>
  </FormControl>
}

export function TitleTiny({ children }: { children: any }) {
  return <Typography variant="subtitle1">
    {children}
  </Typography>
}

export function TitleSmall({ children }: { children: any }) {
  return <Typography variant="h6">
    <b>{children}</b>
  </Typography>
}

export function Title({ children }: { children: any }) {
  return <Typography variant="h5">
    <b>{children}</b>
  </Typography>
}

export function CustomCheckbox({ valfunc, name, initial_value, onchange }: { valfunc?: [value: boolean, func: (str: boolean) => void], name?: string, initial_value?: boolean, onchange?: (checked: boolean) => void }) {
  const ref_checkbox = React.useRef(null);

  const control = <Checkbox checked={initial_value ? true : (valfunc ? valfunc[0] : false)} value={valfunc ? valfunc[0] : initial_value} onChange={(e) => {
    if (valfunc)
      valfunc[1](e.target.checked)
    if (onchange)
      onchange(e.target.checked);
  }} />

  if (name) {
    return <FormControlLabel control={control} label={name} />
  }
  else {
    return <FormControl>
      {control}
    </FormControl>
  }
}

export function RadioSelect({ names, change_callback: state_change }: { names: Array<[name: string, display_text: string]>, change_callback: (val: string) => void }) {
  let states: any[string] = [];

  for (let cell of names) {
    states[cell[0]] = useState<boolean>(false);
  }

  let checkboxes = new Array<JSX.Element>();

  for (let cell of names) {
    let name = cell[0];
    let display_name = cell[1];
    let a = states[name];
    checkboxes.push(<CustomCheckbox key={display_name} name={display_name} valfunc={a} onchange={() => {
      //Disable all
      for (let other_cell of names) {
        states[other_cell[0]][1](false); //Set all to false
      }

      //Enable this
      states[name][1](true);
      state_change(name);
    }} />)
  }

  return <BoxDown>
    {checkboxes}
  </BoxDown>
}

export function FormErrorText() {
  const [error_msg, setErrorMsg] = useState("");

  let msg: JSX.Element | undefined = undefined;

  if (error_msg && error_msg.length > 0) {
    msg = <FormHelperText error={true} style={{
      fontWeight: "bold",
      color: "#F99",
      fontSize: "1.1em"
    }}>{error_msg}</FormHelperText>
  }

  return {
    launch: async (callback: any) => {
      try {
        await callback();
        setErrorMsg("");
      }
      catch (e) {
        setErrorMsg(e ? e.toString() : "Unknown error");
      }
    },
    msg: msg,
    setErrorMsg: setErrorMsg
  }
}
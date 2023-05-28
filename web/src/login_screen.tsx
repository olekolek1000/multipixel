import React, { useEffect, useState } from "react";
import style_login from "./login_screen.scss";
import { BoxDown, CustomTextField, FormControlSpaced, FormErrorText, TitleSmall, TitleTiny } from "./gui_custom"
import { Button } from "@mui/material";
import { Multipixel } from "./multipixel";
import { globals } from ".";

export function LoginScreen({ initial_error_text }: { initial_error_text?: string }) {
  const [username_val, setUsername] = useState("");
  const [room_name_val, setRoomName] = useState("main");
  const [connecting, setConnecting] = useState(false);

  const error = FormErrorText();

  useEffect(() => {
    let storage_nickname = localStorage.getItem("nickname");
    if (storage_nickname)
      setUsername(storage_nickname);

    let storage_room_name = localStorage.getItem("room_name");
    if (storage_room_name)
      setRoomName(storage_room_name);

    if (initial_error_text)
      error.setErrorMsg(initial_error_text);
  }, []);

  const start = () => {
    error.launch(async () => {
      setConnecting(true);
      try {
        let username = username_val.trim();
        let room_name = room_name_val.trim();

        if (username.length == 0) {
          throw new Error("Nickname cannot be empty");
        }
        else if (username.length > 32) {
          throw new Error("Too long nickname");
        }
        else if (room_name.length < 3) {
          throw new Error("Too short room name");
        }
        else if (room_name.length > 32) {
          throw new Error("Too long room name");
        }

        localStorage.setItem("nickname", username);
        localStorage.setItem("room_name", room_name);

        const perform = () => {
          return new Promise((resolve, reject) => {
            let multipixel = new Multipixel({
              host: "ws://127.0.0.1:59900",
              nickname: username,
              room_name: room_name,
              connection_callback: (error_str) => {
                if (error_str) {
                  globals.setState(<LoginScreen initial_error_text={error_str} />)
                  reject(error_str);
                  return;
                }
                else {
                  resolve(undefined);
                }
              }
            })
          });
        }

        await perform();
        setConnecting(false);
      }
      catch (e) {
        setConnecting(false);
        throw e;
      }
    })
  };

  let content: JSX.Element;
  if (connecting) {
    content = <TitleTiny>Connecting...</TitleTiny>
  }
  else {
    content = <>
      <CustomTextField required label="Username" valfunc={[username_val, setUsername]} onReturnPress={start} />
      <CustomTextField required label="Room name" valfunc={[room_name_val, setRoomName]} onReturnPress={start} />
      <Button onClick={start}>START</Button>
      <TitleTiny>
        A program that allows you to draw on an infinitely large canvas - in multiplayer!
      </TitleTiny>
      {error.msg}
    </>
  }

  return <div className={style_login.parent}>
    <BoxDown center_vert center_horiz>
      <FormControlSpaced center_horiz>
        <video width="512" height="128" autoPlay muted={true}>
          <source src="public/logo.webm" type="video/webm" />
          Logo
        </video>
        {content}
        <TitleTiny>
          <b>Created by oo8dev and KuczaRacza</b>
        </TitleTiny>
      </FormControlSpaced>
    </BoxDown>
  </div>;
}
import React, { useEffect, useState } from "react";
import style_login from "./login_screen.module.scss";
import { BoxDown, Button, LabeledTextField, FormErrorText, TitleTiny } from "./gui_custom"
import { Multipixel } from "./multipixel";
import { globals } from ".";

import * as defines from "./global_defines";
import { JSX } from "react/jsx-runtime";


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

				localStorage.setItem("nickname", username);
				localStorage.setItem("room_name", room_name);

				const perform = () => {
					return new Promise((resolve, reject) => {
						new Multipixel({
							host: defines.connect_url,
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
			<LabeledTextField required label="Username" valfunc={[username_val, setUsername]} onReturnPress={start} />
			<LabeledTextField required label="Room name" valfunc={[room_name_val, setRoomName]} onReturnPress={start} />
			<Button on_click={start}>Join</Button>
			<TitleTiny>
				BETA VERSION, branch {defines.commit_branch}, git hash {defines.commit_hash}
			</TitleTiny>
			{error.msg}
		</>
	}

	const ref_video = React.useRef(null);
	const [logo_container, setLogoContainer] = useState(<video ref={ref_video} width="512" height="128" autoPlay muted playsInline={true}>
		<source src="logo.webm" type="video/webm" />
		Logo
	</video>);

	useEffect(() => {
		if (!ref_video.current) {
			return;
		}
		const video = ref_video.current as HTMLVideoElement;
		video.muted = true;

		video.play().catch((e) => {
			console.log("Cannot play video:", e);
			console.log("Falling back to plain logo image");
			setLogoContainer(<img src="logo.webp" width={512} height={128} />);
		});
	}, []);

	return <div className={style_login.background}>
		<BoxDown center_vert center_horiz>
			<div className={style_login.content_box} >
				<BoxDown>
					{logo_container}
					{content}
				</BoxDown>
			</div>
		</BoxDown>
	</div>;
}
import React, { type ReactNode, useEffect, useState } from "react";
import style_login from "./login_screen.module.scss";
import { BoxDown, Button, LabeledTextField, FormErrorText, TitleTiny } from "../../gui_custom"
import { Multipixel } from "../../multipixel";
import { globals } from "../..";

import * as defines from "../../global_defines";
import { useLocalState } from "../../utils/useLocalState";
import { parseAsString, useQueryState } from "nuqs";


export function LoginScreen({ initial_error_text }: { initial_error_text?: string }) {
	const [username, setUsername] = useLocalState("nickname", "");
	const [roomName, setRoomName] = useQueryState("room_name", parseAsString.withDefault("main"));
	const [connecting, setConnecting] = useState(false);

	const error = FormErrorText();

	useEffect(() => {
		if (initial_error_text)
			error.setErrorMsg(initial_error_text);
	}, []);

	const start = () => {
		error.launch(async () => {
			setConnecting(true);
			try {

				const perform = () => {
					return new Promise((resolve, reject) => {
						new Multipixel({
							connect_params: {
								host: defines.connect_url,
								nickname: username.trim(),
								room_name: roomName.trim(),
							},
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

	let content: ReactNode;
	if (connecting) {
		content = <TitleTiny>Connecting...</TitleTiny>
	}
	else {
		content = <>
			<LabeledTextField required label="Username" valfunc={[username, setUsername]} onReturnPress={start} />
			<LabeledTextField required label="Room name" valfunc={[roomName, setRoomName]} onReturnPress={start} />
			<Button on_click={start}>Join</Button>
			<TitleTiny>
				BETA VERSION, branch {defines.commit_branch}, git hash {defines.commit_hash}
			</TitleTiny>
			{error.msg}
		</>
	}

	const ref_video = React.useRef(null);
	const [logo_container, setLogoContainer] = useState(
		<video ref={ref_video} width="512" height="128" autoPlay muted playsInline={true}>
			<source src="logo.webm" type="video/webm" />
			Logo
		</video>
	);

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
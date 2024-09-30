import React, { Dispatch, SetStateAction, useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { LoginScreen } from "./login_screen";
import style from "./style.scss"

export namespace globals {
	export let state: JSX.Element | null = null;
	export let setState: React.Dispatch<React.SetStateAction<JSX.Element>>;
	export let root: HTMLElement;
}


function MainApp({ }: {}) {
	const [state, setState] = useState(<LoginScreen />);
	globals.state = state;
	globals.setState = setState;

	return state;
}

window.onload = function () {
	globals.root = document.getElementById('root')!;
	const root = ReactDOM.createRoot(globals.root);
	root.render(<MainApp />);
}

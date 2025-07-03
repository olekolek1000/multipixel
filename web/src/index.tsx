import React, { type FC, type ReactNode, useState } from "react";
import ReactDOM from "react-dom/client";
import { LoginScreen } from "./views/login/login_screen";
import { NuqsAdapter } from 'nuqs/adapters/react'

import './styles.css';

export namespace globals {
	export let state: ReactNode = null;
	export let setState: React.Dispatch<React.SetStateAction<ReactNode>>;
	export let root: HTMLElement;
}


const MainApp: FC = () => {
	const [state, setState] = useState<ReactNode>(<LoginScreen />);
	globals.state = state;
	globals.setState = setState;

	return state;
}

globals.root ??= document.getElementById('root')!;

ReactDOM.createRoot(globals.root).render(
	<NuqsAdapter>
		<MainApp />
	</NuqsAdapter>
);

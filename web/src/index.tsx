import React, { Dispatch, SetStateAction, useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { LoginScreen } from "./login_screen";
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';

export namespace globals {
  export let state: JSX.Element | null = null;
  export let setState: React.Dispatch<React.SetStateAction<JSX.Element>>;
  export let root: HTMLElement;
}

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
  },
});

function MainApp({ }: {}) {
  const [state, setState] = useState(<LoginScreen />);
  globals.state = state;
  globals.setState = setState;

  return state;
}

window.onload = function () {
  globals.root = document.getElementById('root')!;
  const root = ReactDOM.createRoot(globals.root);
  root.render(<ThemeProvider theme={darkTheme}>
    <CssBaseline />
    <MainApp />
  </ThemeProvider>);
}

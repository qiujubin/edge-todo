/* 边芽 EdgeSprout 入口 */
import { render } from "solid-js/web";

import "@fontsource/chakra-petch/400.css";
import "@fontsource/chakra-petch/500.css";
import "@fontsource/chakra-petch/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/600.css";
import "@fontsource/jetbrains-mono/700.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/app.css";
import { App } from "./App";

render(() => <App />, document.getElementById("root")!);

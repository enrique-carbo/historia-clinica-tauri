import "./index.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { APP_NAME } from "./utils/branding";

document.title = APP_NAME;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

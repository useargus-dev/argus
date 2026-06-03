import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app";
import "../bones/registry";
import "../styles/globals.css";
import { initTheme } from "@/core/theme";

initTheme();

document.addEventListener("contextmenu", (e) => e.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

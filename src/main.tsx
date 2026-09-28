import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

window.addEventListener("contextmenu", (event) => {
  if (!(event.target instanceof HTMLElement) || !event.target.closest(".select-text")) event.preventDefault();
});

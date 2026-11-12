import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@aether/design-system/tokens.css";
import "@aether/design-system/components.css";
import "@aether/ui/ui.css";
import "./site.css";
import { App } from "./App";

const root = document.getElementById("root");
if (root === null) throw new Error("missing #root");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

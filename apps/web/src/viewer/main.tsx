import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Viewer } from "./Viewer";
import "./viewer.css";

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <Viewer />
    </StrictMode>,
  );
}

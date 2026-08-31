import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { surface } from "./lib/surface";
import "./index.css";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");

createRoot(root).render(
  <StrictMode>
    <App surface={surface()} />
  </StrictMode>,
);

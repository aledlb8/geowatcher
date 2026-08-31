import { Configurator } from "./config/Configurator";
import type { Surface } from "./lib/surface";
import { Widget } from "./widget/Widget";

/** One bundle serves both windows; the window label decides which one renders. */
export default function App({ surface }: { surface: Surface }) {
  return surface === "widget" ? <Widget /> : <Configurator />;
}

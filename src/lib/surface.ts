export type Surface = "app" | "widget";

/**
 * Which window this bundle is rendering. Set on <html> by the inline script in
 * index.html, which is the single place the widget's window label is checked.
 */
export function surface(): Surface {
  return document.documentElement.dataset.surface === "widget" ? "widget" : "app";
}

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getSettings, getUserCss, onHover, onUserCss, openChannel, resizeWidget } from "../lib/api";
import { BLANK, isLive, type Live, type Settings } from "../lib/types";
import { useWatcher } from "../lib/useWatcher";
import { Card, type WidgetRow, widgetVars } from "./Card";

function note(entry: Live): string {
  switch (entry.status.kind) {
    case "calibrating":
      return "finding bar";
    case "connecting":
      return "connecting";
    case "error":
      return "error";
    case "watching":
      return "waiting";
    default:
      return "offline";
  }
}

export function Widget() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [userCss, setUserCss] = useState("");
  const [hovered, setHovered] = useState(false);
  const live = useWatcher();
  const card = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void getSettings().then(setSettings);
    void getUserCss().then(setUserCss);

    const hover = onHover(setHovered);
    const css = onUserCss(setUserCss);
    // Appearance changes arrive by re-reading settings; cheap at a human pace.
    const timer = setInterval(() => void getSettings().then(setSettings), 1500);

    return () => {
      void hover.then((un) => un());
      void css.then((un) => un());
      clearInterval(timer);
    };
  }, []);

  const appearance = settings?.appearance;

  // Report the rendered height so the window hugs its content instead of
  // leaving a transparent dead zone that still eats the corner.
  useLayoutEffect(() => {
    const el = card.current;
    if (!el || !appearance) return;
    // offsetHeight, not contentRect: the latter excludes padding and border.
    const observer = new ResizeObserver(() => {
      if (el.offsetHeight > 0) void resizeWidget(Math.ceil(el.offsetHeight));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [appearance]);

  if (!settings || !appearance) return null;

  const rows: WidgetRow[] = settings.channels
    .filter((channel) => channel.enabled)
    .map((channel) => ({ login: channel.login, entry: live[channel.login] ?? BLANK }))
    .filter(({ entry }) => appearance.showOffline || isLive(entry.status))
    .map(({ login, entry }) => ({
      login,
      percent: entry.current,
      note: note(entry),
      history: entry.history,
    }));

  const running = rows.some((row) => row.percent !== null);

  return (
    <div className="gw-root" style={widgetVars(appearance)} data-hovered={hovered || undefined}>
      {/* The escape hatch: anything the controls do not cover. */}
      {userCss && <style>{userCss}</style>}

      <Card
        appearance={appearance}
        rows={rows}
        collapsed={appearance.collapseWhenIdle && !running && !hovered}
        onOpen={(login) => void openChannel(login)}
        cardRef={card}
      />
    </div>
  );
}

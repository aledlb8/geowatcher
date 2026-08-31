import { type CSSProperties, useCallback, useState } from "react";
import type { Appearance, Attempt } from "../lib/types";
import { Card, Segmented } from "../ui/primitives";
import { Card as WidgetCard, type WidgetRow, widgetVars } from "../widget/Card";

const SCREEN = { width: 1920, height: 1200 };

const CORNER_LABEL: Record<Appearance["corner"], string> = {
  topLeft: "top left",
  topRight: "top right",
  bottomLeft: "bottom left",
  bottomRight: "bottom right",
};

// Fixed sample data: the point is to judge the look, and a number that keeps
// moving makes colours harder to compare.
const SAMPLE: WidgetRow[] = [
  { login: "zoinkgd", percent: 87.42, note: "waiting", history: attempts(26, 7) },
  { login: "kamensh1k0", percent: 31.06, note: "waiting", history: attempts(20, 3) },
  { login: "spotz_live", percent: null, note: "waiting", history: attempts(14, 11) },
];

/** Deterministic sample attempts: mostly early deaths, a few deep runs. */
function attempts(count: number, seed: number): Attempt[] {
  const out: Attempt[] = [];
  let state = seed * 9301 + 49297;
  for (let i = 0; i < count; i++) {
    state = (state * 9301 + 49297) % 233280;
    const roll = state / 233280;
    out.push({
      id: seed * 1000 + i,
      peak: roll > 0.86 ? 30 + roll * 65 : roll * 26,
      fromStart: roll > 0.12,
    });
  }
  return out;
}

export function Preview({ appearance }: { appearance: Appearance }) {
  const [mode, setMode] = useState<"screen" | "actual">("screen");
  const [scale, setScale] = useState(0.2);

  // Ref callback rather than an effect: the frame only exists in screen mode.
  const frame = useCallback((el: HTMLDivElement | null) => {
    if (!el) return;
    const observer = new ResizeObserver(() => setScale(el.clientWidth / SCREEN.width));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const rows = appearance.showOffline ? SAMPLE : SAMPLE.filter((row) => row.percent !== null);
  const right = appearance.corner === "topRight" || appearance.corner === "bottomRight";
  const bottom = appearance.corner === "bottomLeft" || appearance.corner === "bottomRight";

  const docked: CSSProperties = {
    position: "absolute",
    width: appearance.width,
    left: right ? undefined : appearance.marginX,
    right: right ? appearance.marginX : undefined,
    top: bottom ? undefined : appearance.marginY,
    bottom: bottom ? appearance.marginY : undefined,
  };

  return (
    <Card
      title="Preview"
      flush
      actions={
        <Segmented
          value={mode}
          onChange={setMode}
          options={[
            { value: "screen", label: "On screen" },
            { value: "actual", label: "Actual size" },
          ]}
        />
      }
      footer={
        <span className="font-mono text-[10px] text-overlay">
          sample data · docks {CORNER_LABEL[appearance.corner]}
          {mode === "screen" && ` · ${Math.round(scale * 100)}%`}
        </span>
      }
    >
      {mode === "screen" ? (
        <div className="relative aspect-[16/10] overflow-hidden bg-crust" ref={frame}>
          <div
            className="absolute top-0 left-0 origin-top-left"
            style={{ width: SCREEN.width, height: SCREEN.height, transform: `scale(${scale})` }}
          >
            <Desktop />
            <div className="gw-root" style={{ ...widgetVars(appearance), ...docked }}>
              <WidgetCard appearance={appearance} rows={rows} />
            </div>
          </div>
        </div>
      ) : (
        <div className="flex overflow-x-auto bg-crust p-6">
          <div
            className="gw-root m-auto"
            style={{ ...widgetVars(appearance), width: appearance.width }}
          >
            <WidgetCard appearance={appearance} rows={rows} />
          </div>
        </div>
      )}
    </Card>
  );
}

/** A tiling-WM desktop, so the widget is judged against what it sits on. */
function Desktop() {
  return (
    <>
      <div className="absolute top-5 right-5 left-5 flex h-13 items-center gap-3 rounded-xl border border-surface0 bg-mantle px-4">
        <Module width={90} />
        <Module width={54} />
        <div className="mx-auto flex gap-3">
          <Module width={70} accent />
          <Module width={40} />
        </div>
        <Module width={64} />
        <Module width={46} />
      </div>

      <Window left={20} width={1060} focused />
      <Window left={1100} width={800} />
    </>
  );
}

function Module({ width, accent }: { width: number; accent?: boolean }) {
  return (
    <span
      className={`block h-6 rounded-md ${accent ? "bg-blue/30" : "bg-surface0"}`}
      style={{ width }}
    />
  );
}

function Window({ left, width, focused }: { left: number; width: number; focused?: boolean }) {
  return (
    <div
      className={`absolute rounded-2xl border-2 bg-base ${focused ? "border-lavender" : "border-surface0"}`}
      style={{ left, width, top: 112, height: 1068 }}
    >
      <div className="flex h-10 items-center gap-2 border-b-2 border-inherit px-5">
        <span className="h-2.5 w-2.5 rounded-full bg-surface1" />
        <span className="h-2.5 w-2.5 rounded-full bg-surface1" />
        <span className="ml-3 h-2 w-40 rounded bg-surface0" />
      </div>
      <div className="flex flex-col gap-6 p-10">
        {[92, 74, 58, 84, 40, 66, 78, 34].map((pct) => (
          <span key={pct} className="h-6 rounded bg-surface0" style={{ width: `${pct}%` }} />
        ))}
      </div>
    </div>
  );
}

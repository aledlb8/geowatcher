import type { ReactNode } from "react";
import type { Appearance, Corner } from "../lib/types";
import { Card, Field, INPUT, Slider, Swatch, Switch } from "../ui/primitives";
import { Preview } from "./Preview";
import { UserCss } from "./UserCss";

const CORNERS: Array<{ value: Corner; x: string; y: string }> = [
  { value: "topLeft", x: "left-[12%]", y: "top-[14%]" },
  { value: "topRight", x: "right-[12%]", y: "top-[14%]" },
  { value: "bottomLeft", x: "left-[12%]", y: "bottom-[14%]" },
  { value: "bottomRight", x: "right-[12%]", y: "bottom-[14%]" },
];

type Props = {
  appearance: Appearance;
  onChange: (next: Appearance) => void;
};

export function WidgetView({ appearance, onChange }: Props) {
  const set = <K extends keyof Appearance>(key: K, value: Appearance[K]) =>
    onChange({ ...appearance, [key]: value });

  return (
    <div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(360px,420px)]">
      <div className="flex flex-col gap-5">
        <Card title="Placement">
          <div className="grid gap-5 sm:grid-cols-[168px_minmax(0,1fr)]">
            <Field label="Corner">
              <div className="grid grid-cols-2 gap-1.5">
                {CORNERS.map((corner) => {
                  const on = appearance.corner === corner.value;
                  return (
                    <button
                      key={corner.value}
                      type="button"
                      aria-pressed={on}
                      aria-label={corner.value}
                      onClick={() => set("corner", corner.value)}
                      className={`relative aspect-[16/10] rounded border transition-colors ${
                        on
                          ? "border-blue bg-blue/10"
                          : "border-surface0 bg-mantle hover:border-surface2"
                      }`}
                    >
                      <span
                        className={`absolute h-[22%] w-[34%] rounded-[2px] ${corner.x} ${corner.y} ${
                          on ? "bg-blue" : "bg-surface1"
                        }`}
                      />
                    </button>
                  );
                })}
              </div>
            </Field>

            <div className="flex flex-col gap-4">
              <Field label="Width" value={`${appearance.width}px`}>
                <Slider
                  min={160}
                  max={640}
                  value={appearance.width}
                  label="Width"
                  onChange={(v) => set("width", v)}
                />
              </Field>
              <Field label="Edge gap across" value={`${appearance.marginX}px`}>
                <Slider
                  min={0}
                  max={200}
                  value={appearance.marginX}
                  label="Edge gap across"
                  onChange={(v) => set("marginX", v)}
                />
              </Field>
              <Field label="Edge gap down" value={`${appearance.marginY}px`}>
                <Slider
                  min={0}
                  max={200}
                  value={appearance.marginY}
                  label="Edge gap down"
                  onChange={(v) => set("marginY", v)}
                />
              </Field>
            </div>
          </div>
        </Card>

        <Card title="Surface">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Opacity" value={appearance.opacity.toFixed(2)}>
              <Slider
                min={0.15}
                max={1}
                step={0.01}
                value={appearance.opacity}
                label="Opacity"
                onChange={(v) => set("opacity", v)}
              />
            </Field>
            <Field label="Corner radius" value={`${appearance.radius}px`}>
              <Slider
                min={0}
                max={40}
                value={appearance.radius}
                label="Corner radius"
                onChange={(v) => set("radius", v)}
              />
            </Field>
            <Field label="Padding" value={`${appearance.padding}px`}>
              <Slider
                min={0}
                max={40}
                value={appearance.padding}
                label="Padding"
                onChange={(v) => set("padding", v)}
              />
            </Field>
            <Field label="Row spacing" value={`${appearance.gap}px`}>
              <Slider
                min={0}
                max={36}
                value={appearance.gap}
                label="Row spacing"
                onChange={(v) => set("gap", v)}
              />
            </Field>
          </div>

          <div className="mt-5 grid gap-4 border-t border-surface0 pt-5 sm:grid-cols-2">
            <Field label="Background">
              <Swatch
                label="Background"
                value={appearance.background}
                onChange={(v) => set("background", v)}
              />
            </Field>
            <Field label="Border">
              <Swatch label="Border" value={appearance.line} onChange={(v) => set("line", v)} />
            </Field>
            <Field label="Text">
              <Swatch label="Text" value={appearance.text} onChange={(v) => set("text", v)} />
            </Field>
            <Field label="Muted text">
              <Swatch label="Muted text" value={appearance.dim} onChange={(v) => set("dim", v)} />
            </Field>
          </div>
        </Card>

        <Card title="Type">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Interface font">
              <input
                className={INPUT}
                value={appearance.fontFamily}
                spellCheck={false}
                aria-label="Interface font"
                onChange={(e) => set("fontFamily", e.currentTarget.value)}
              />
            </Field>
            <Field label="Numeral font">
              <input
                className={INPUT}
                value={appearance.numeralFamily}
                spellCheck={false}
                aria-label="Numeral font"
                onChange={(e) => set("numeralFamily", e.currentTarget.value)}
              />
            </Field>
          </div>
          <div className="mt-4">
            <Field label="Text size" value={`${appearance.fontSize}px`}>
              <Slider
                min={8}
                max={24}
                value={appearance.fontSize}
                label="Text size"
                onChange={(v) => set("fontSize", v)}
              />
            </Field>
          </div>
        </Card>

        <Card title="Progress ramp">
          <div
            className="h-4 rounded"
            style={{
              background: `linear-gradient(90deg, ${appearance.ramp.low} 0%, ${appearance.ramp.mid} 45%, ${appearance.ramp.high} 72%, ${appearance.ramp.peak} 92%)`,
            }}
          />
          <div className="mt-1.5 flex justify-between font-mono text-[9px] tabular-nums text-overlay">
            <span>0%</span>
            <span>45%</span>
            <span>72%</span>
            <span>100%</span>
          </div>
          <p className="mt-4 text-[12.5px] leading-relaxed text-overlay">
            Colour carries how far a run has got, so a glance is enough. These four stops blend
            across the bar, the percentage and every history tick.
          </p>
          <div className="mt-5 grid gap-4 border-t border-surface0 pt-5 sm:grid-cols-2">
            <Field label="Early">
              <Swatch
                label="Early"
                value={appearance.ramp.low}
                onChange={(v) => set("ramp", { ...appearance.ramp, low: v })}
              />
            </Field>
            <Field label="Mid run">
              <Swatch
                label="Mid run"
                value={appearance.ramp.mid}
                onChange={(v) => set("ramp", { ...appearance.ramp, mid: v })}
              />
            </Field>
            <Field label="Past threshold">
              <Swatch
                label="Past threshold"
                value={appearance.ramp.high}
                onChange={(v) => set("ramp", { ...appearance.ramp, high: v })}
              />
            </Field>
            <Field label="Near the end">
              <Swatch
                label="Near the end"
                value={appearance.ramp.peak}
                onChange={(v) => set("ramp", { ...appearance.ramp, peak: v })}
              />
            </Field>
          </div>
        </Card>

        <Card title="Contents">
          <div className="flex flex-col gap-4">
            <Switch
              label="Progress bar"
              checked={appearance.showBar}
              onChange={(v) => set("showBar", v)}
            />
            {appearance.showBar && (
              <Indent>
                <Field label="Bar height" value={`${appearance.barHeight}px`}>
                  <Slider
                    min={1}
                    max={16}
                    value={appearance.barHeight}
                    label="Bar height"
                    onChange={(v) => set("barHeight", v)}
                  />
                </Field>
              </Indent>
            )}
            <Switch
              label="Attempt history"
              hint="One tick per attempt, height by how far it got. Start Pos runs are drawn faint."
              checked={appearance.showHistory}
              onChange={(v) => set("showHistory", v)}
            />
            {appearance.showHistory && (
              <Indent>
                <div className="grid gap-4 sm:grid-cols-2">
                  <Field label="Attempts shown" value={String(appearance.historyLength)}>
                    <Slider
                      min={6}
                      max={80}
                      value={appearance.historyLength}
                      label="Attempts shown"
                      onChange={(v) => set("historyLength", v)}
                    />
                  </Field>
                  <Field label="History height" value={`${appearance.historyHeight}px`}>
                    <Slider
                      min={6}
                      max={48}
                      value={appearance.historyHeight}
                      label="History height"
                      onChange={(v) => set("historyHeight", v)}
                    />
                  </Field>
                </div>
              </Indent>
            )}
            <Switch
              label="List offline channels"
              checked={appearance.showOffline}
              onChange={(v) => set("showOffline", v)}
            />
          </div>
        </Card>

        <Card title="Behaviour">
          <div className="flex flex-col gap-4">
            <Switch
              label="Shrink when nothing is running"
              hint="Collapses to a small pill, expands on hover or when a run starts."
              checked={appearance.collapseWhenIdle}
              onChange={(v) => set("collapseWhenIdle", v)}
            />
            <Switch
              label="Let clicks pass through"
              hint="The widget ignores the mouse until you hover it."
              checked={appearance.clickThrough}
              onChange={(v) => set("clickThrough", v)}
            />
            <Switch
              label="Keep above other windows"
              checked={appearance.alwaysOnTop}
              onChange={(v) => set("alwaysOnTop", v)}
            />
            <Switch
              label="Show the widget"
              checked={appearance.enabled}
              onChange={(v) => set("enabled", v)}
            />
          </div>
        </Card>

        <UserCss />
      </div>

      <div className="xl:sticky xl:top-0">
        <Preview appearance={appearance} />
      </div>
    </div>
  );
}

/** Sub-controls belonging to the toggle above them. */
function Indent({ children }: { children: ReactNode }) {
  return <div className="border-l border-surface0 pl-4 ml-3.5">{children}</div>;
}

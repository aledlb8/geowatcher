import type { CSSProperties, Ref } from "react";
import { rampColor, rampTrack } from "../lib/ramp";
import type { Appearance, Attempt } from "../lib/types";
import { History } from "./History";

export type WidgetRow = {
  login: string;
  percent: number | null;
  /** Shown in place of the percentage when there is no run in progress. */
  note: string;
  history: Attempt[];
};

/** Every knob in the Widget tab, handed to CSS as custom properties. */
export function widgetVars(appearance: Appearance): CSSProperties {
  return {
    "--gw-bg": appearance.background,
    "--gw-panel": appearance.panel,
    "--gw-line": appearance.line,
    "--gw-text": appearance.text,
    "--gw-dim": appearance.dim,
    "--gw-radius": `${appearance.radius}px`,
    "--gw-pad": `${appearance.padding}px`,
    "--gw-gap": `${appearance.gap}px`,
    "--gw-font": `"${appearance.fontFamily}", system-ui, sans-serif`,
    "--gw-numerals": `"${appearance.numeralFamily}", ui-monospace, monospace`,
    "--gw-size": `${appearance.fontSize}px`,
    "--gw-opacity": String(appearance.opacity),
    "--gw-track": rampTrack(appearance.ramp),
  } as CSSProperties;
}

type Props = {
  appearance: Appearance;
  rows: WidgetRow[];
  /** Collapsed to a pill: nothing is running and the widget is not hovered. */
  collapsed?: boolean;
  onOpen?: (login: string) => void;
  cardRef?: Ref<HTMLDivElement>;
};

/**
 * The card itself, shared by the live widget and the configurator preview so
 * the two can never drift apart.
 */
export function Card({ appearance, rows, collapsed, onOpen, cardRef }: Props) {
  const idle = collapsed || rows.length === 0;

  return (
    <div className="gw-card" ref={cardRef}>
      {idle ? (
        <div className="gw-pill">
          <Mark colour={rows.length > 0 ? appearance.ramp.low : appearance.dim} />
          <span className="gw-pill-text">
            {rows.length === 0 ? "nobody live" : `${rows.length} live`}
          </span>
        </div>
      ) : (
        <div className="gw-rows">
          {rows.map((row) => {
            const percent = Math.min(row.percent ?? 0, 100);
            const colour =
              row.percent === null ? appearance.dim : rampColor(percent, appearance.ramp);

            return (
              <button
                key={row.login}
                type="button"
                className="gw-row"
                tabIndex={onOpen ? undefined : -1}
                title={onOpen ? `Open ${row.login}` : undefined}
                onClick={onOpen ? () => onOpen(row.login) : undefined}
              >
                <span className="gw-head">
                  <Mark colour={colour} />
                  <span className="gw-name">{row.login}</span>
                  {row.percent === null ? (
                    <span className="gw-note">{row.note}</span>
                  ) : (
                    <span className="gw-value" style={{ color: colour }}>
                      {row.percent.toFixed(2)}
                      <span className="gw-unit">%</span>
                    </span>
                  )}
                </span>

                {appearance.showBar && (
                  <span className="gw-bar" style={{ height: appearance.barHeight }}>
                    <span
                      className="gw-fill"
                      style={{ width: `${percent}%`, background: colour }}
                    />
                  </span>
                )}

                {appearance.showHistory && (
                  <History
                    attempts={row.history}
                    ramp={appearance.ramp}
                    height={appearance.historyHeight}
                    limit={appearance.historyLength}
                  />
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** Diamond mark, echoing the shape the game is built out of. */
function Mark({ colour }: { colour: string }) {
  return <span className="gw-mark" style={{ background: colour }} aria-hidden="true" />;
}

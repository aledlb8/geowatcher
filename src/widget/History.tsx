import { rampColor } from "../lib/ramp";
import type { Attempt, Ramp } from "../lib/types";

type Props = {
  attempts: Attempt[];
  ramp: Ramp;
  height: number;
  limit: number;
};

/** One tick per finished attempt, height by peak reached. */
export function History({ attempts, ramp, height, limit }: Props) {
  const shown = attempts.slice(-limit);

  // Keep the row's rhythm before any attempt has finished, but draw nothing:
  // a rule here reads as a divider between channels.
  if (shown.length === 0) {
    return <div className="gw-history" style={{ height }} />;
  }

  return (
    <div className="gw-history" style={{ height }}>
      {shown.map((attempt) => {
        const colour = rampColor(attempt.peak, ramp);
        // Square root, not linear: most attempts die in the first few percent,
        // and a linear scale floors them all to the same stub.
        const scaled = Math.sqrt(Math.min(attempt.peak, 100) / 100);
        return (
          <span
            key={attempt.id}
            className="gw-tick"
            style={{
              height: Math.max(2, Math.round(scaled * height)),
              background: colour,
              // Start Pos runs stay solid but faint; an outline reads as an
              // empty box at this size and wrecks the silhouette.
              opacity: attempt.fromStart ? 1 : 0.28,
            }}
          />
        );
      })}
    </div>
  );
}

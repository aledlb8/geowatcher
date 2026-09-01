import { useEffect, useState } from "react";
import { onUpdate } from "./api";
import { BLANK, type Live, type Update } from "./types";

/** Attempts kept per channel; the widget shows at most `historyLength` of them. */
const HISTORY_CAP = 120;

let nextAttemptId = 0;

/**
 * Accumulates live channel state from watcher events.
 *
 * The attempt history is built here rather than in Rust: the backend already
 * reports every finished run, so the strip is just those peaks in order.
 */
export function useWatcher(): Record<string, Live> {
  const [live, setLive] = useState<Record<string, Live>>({});

  useEffect(() => {
    const pending = onUpdate((update: Update) => {
      setLive((prev) => {
        const entry = prev[update.login] ?? BLANK;
        switch (update.type) {
          case "status":
            return { ...prev, [update.login]: { ...entry, status: update.status, current: null } };

          case "progress":
            return {
              ...prev,
              [update.login]: {
                ...entry,
                current: update.value,
                fromStart: update.fromStart,
              },
            };

          case "runEnded":
            return {
              ...prev,
              [update.login]: {
                ...entry,
                current: null,
                best: update.fromStart ? Math.max(entry.best, update.peak) : entry.best,
                history: [
                  ...entry.history,
                  { id: nextAttemptId++, peak: update.peak, fromStart: update.fromStart },
                ].slice(-HISTORY_CAP),
              },
            };

          default:
            return prev;
        }
      });
    });

    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, []);

  return live;
}

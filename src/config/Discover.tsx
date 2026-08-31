import { useEffect, useState } from "react";
import { discover } from "../lib/api";
import { Icon } from "../ui/Icon";
import { Card } from "../ui/primitives";

type Found = { login: string; viewers: number };

export function Discover({ known, onAdd }: { known: string[]; onAdd: (login: string) => void }) {
  const [rows, setRows] = useState<Found[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    void discover(12)
      .then((found) => {
        if (!cancelled) setRows(found);
      })
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const available = rows.filter((row) => !known.includes(row.login));

  return (
    <Card
      title="Live now in Geometry Dash"
      note={loading ? "scanning" : `${available.length} available`}
    >
      {loading ? (
        <p className="text-[12.5px] text-overlay">Asking Twitch who is streaming…</p>
      ) : available.length === 0 ? (
        <p className="text-[12.5px] text-overlay">
          {rows.length === 0
            ? "Nobody is streaming Geometry Dash right now."
            : "You already watch everyone who is live."}
        </p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2">
          {available.map((row) => (
            <button
              key={row.login}
              type="button"
              onClick={() => onAdd(row.login)}
              className="group flex items-center gap-2 rounded-md border border-surface0 bg-mantle px-3 py-2 text-left transition-colors hover:border-blue hover:bg-surface0/50"
            >
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-muted group-hover:text-fg">
                {row.login}
              </span>
              <span className="font-mono text-[10px] tabular-nums text-overlay">
                {formatViewers(row.viewers)}
              </span>
              <span className="text-overlay group-hover:text-blue">
                <Icon name="plus" size={13} />
              </span>
            </button>
          ))}
        </div>
      )}
    </Card>
  );
}

function formatViewers(count: number): string {
  return count >= 1000 ? `${(count / 1000).toFixed(1)}k` : String(count);
}

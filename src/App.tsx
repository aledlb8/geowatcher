import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

// Placeholder UI. It exercises the backend commands and events so the watcher
// can be verified end to end; the real interface comes later.

type Status =
  | { kind: "offline" }
  | { kind: "connecting" }
  | { kind: "calibrating" }
  | { kind: "watching" }
  | { kind: "error"; message: string };

type Update =
  | { type: "status"; login: string; status: Status }
  | { type: "progress"; login: string; value: number; fromStart: boolean }
  | { type: "alert"; login: string; value: number; streamUrl: string | null }
  | { type: "runEnded"; login: string; peak: number; fromStart: boolean };

type Channel = {
  login: string;
  alert_at: number | null;
  enabled: boolean;
};

type Settings = {
  channels: Channel[];
  alert_at: number;
};

type Live = {
  status: Status;
  current: number | null;
  best: number;
};

const BLANK: Live = { status: { kind: "offline" }, current: null, best: 0 };

function label(status: Status): string {
  return status.kind === "error" ? `error: ${status.message}` : status.kind;
}

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [live, setLive] = useState<Record<string, Live>>({});
  const [input, setInput] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setSettings(await invoke<Settings>("get_settings"));
      setProblem(null);
    } catch (err) {
      setProblem(String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const pending = listen<Update>("watcher://update", ({ payload }) => {
      setLive((prev) => {
        const entry = prev[payload.login] ?? BLANK;
        switch (payload.type) {
          case "status":
            return { ...prev, [payload.login]: { ...entry, status: payload.status } };
          case "progress":
            return { ...prev, [payload.login]: { ...entry, current: payload.value } };
          case "runEnded":
            return {
              ...prev,
              [payload.login]: {
                ...entry,
                current: null,
                best: payload.fromStart ? Math.max(entry.best, payload.peak) : entry.best,
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

  async function addChannel(event: React.FormEvent) {
    event.preventDefault();
    if (!input.trim()) return;
    try {
      await invoke("add_channel", { login: input });
      setInput("");
      await refresh();
    } catch (err) {
      setProblem(String(err));
    }
  }

  async function removeChannel(login: string) {
    try {
      await invoke("remove_channel", { login });
      await refresh();
    } catch (err) {
      setProblem(String(err));
    }
  }

  return (
    <main className="min-h-screen bg-zinc-950 p-8 text-zinc-100">
      <h1 className="text-2xl font-semibold tracking-tight">GeoWatcher</h1>
      <p className="mt-1 text-sm text-zinc-500">
        Backend smoke test. The real interface comes later.
      </p>

      <form onSubmit={addChannel} className="mt-6 flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.currentTarget.value)}
          placeholder="twitch login, e.g. zoinkgd"
          className="w-72 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm outline-none focus:border-sky-500"
        />
        <button
          type="submit"
          className="rounded-lg bg-sky-600 px-4 py-2 text-sm font-medium hover:bg-sky-500"
        >
          Watch
        </button>
      </form>

      {problem && <p className="mt-4 text-sm text-red-400">{problem}</p>}

      <table className="mt-6 w-full max-w-2xl text-left text-sm">
        <thead className="text-zinc-500">
          <tr>
            <th className="py-2 font-medium">Channel</th>
            <th className="py-2 font-medium">Status</th>
            <th className="py-2 font-medium">Current</th>
            <th className="py-2 font-medium">Best</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {settings?.channels.map((channel) => {
            const state = live[channel.login] ?? BLANK;
            return (
              <tr key={channel.login} className="border-t border-zinc-800">
                <td className="py-2 font-medium">{channel.login}</td>
                <td className="py-2 text-zinc-400">{label(state.status)}</td>
                <td className="py-2 tabular-nums">
                  {state.current === null ? "—" : `${state.current.toFixed(2)}%`}
                </td>
                <td className="py-2 tabular-nums">{state.best.toFixed(2)}%</td>
                <td className="py-2 text-right">
                  <button
                    type="button"
                    onClick={() => void invoke("open_channel", { login: channel.login })}
                    className="mr-3 text-sky-400 hover:underline"
                  >
                    open
                  </button>
                  <button
                    type="button"
                    onClick={() => void removeChannel(channel.login)}
                    className="text-zinc-500 hover:text-red-400"
                  >
                    remove
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>

      {settings?.channels.length === 0 && (
        <p className="mt-4 text-sm text-zinc-500">No channels yet.</p>
      )}
    </main>
  );
}

export default App;

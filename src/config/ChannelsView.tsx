import { useState } from "react";
import { addChannel, removeChannel, setEnabled, setSampling, setThreshold } from "../lib/api";
import { rampColor } from "../lib/ramp";
import { BLANK, type Channel, type Live, type Ramp, type Settings } from "../lib/types";
import { Button, Card, Field, HEADING, IconButton, INPUT, Slider } from "../ui/primitives";
import { History } from "../widget/History";
import { Discover } from "./Discover";

/** Shared by the header and every row, so the columns actually line up. */
const COLUMNS = "grid grid-cols-[minmax(104px,1.1fr)_84px_minmax(148px,1.7fr)_124px_60px] gap-4";

type Props = {
  settings: Settings;
  live: Record<string, Live>;
  onProblem: (message: string | null) => void;
  onChanged: () => void;
};

export function ChannelsView({ settings, live, onProblem, onChanged }: Props) {
  const [input, setInput] = useState("");

  async function add(login: string) {
    const clean = login.trim().toLowerCase();
    if (!clean) return;
    try {
      await addChannel(clean);
      setInput("");
      onProblem(null);
      onChanged();
    } catch (err) {
      onProblem(String(err));
    }
  }

  const running = settings.channels.filter(
    (c) => c.enabled && (live[c.login] ?? BLANK).current !== null,
  ).length;

  return (
    <div className="flex max-w-[920px] flex-col gap-5">
      <Card
        title="Watchlist"
        note={`${running} running · ${settings.channels.length} tracked`}
        flush
      >
        <form
          className="flex gap-2 border-b border-surface0 bg-mantle px-5 py-3"
          onSubmit={(e) => {
            e.preventDefault();
            void add(input);
          }}
        >
          <input
            className={INPUT}
            value={input}
            onChange={(e) => setInput(e.currentTarget.value)}
            placeholder="Twitch login, e.g. zoinkgd"
            spellCheck={false}
            autoComplete="off"
            aria-label="Twitch login"
          />
          <Button type="submit" variant="primary" icon="plus" disabled={!input.trim()}>
            Add channel
          </Button>
        </form>

        {settings.channels.length === 0 ? (
          <p className="px-5 py-12 text-center text-[13px] text-overlay">
            Nothing on the watchlist yet. Add a Twitch login above, or pick someone from who is live
            below.
          </p>
        ) : (
          <>
            <div className={`${COLUMNS} border-b border-surface0 bg-mantle px-5 py-2`}>
              <span className={HEADING}>Channel</span>
              <span className={HEADING}>Status</span>
              <span className={HEADING}>Progress</span>
              <span className={HEADING}>Alert at</span>
              <span />
            </div>

            {settings.channels.map((channel) => (
              <Row
                key={channel.login}
                channel={channel}
                entry={live[channel.login] ?? BLANK}
                fallback={settings.alert_at}
                ramp={settings.appearance.ramp}
                onChanged={onChanged}
                onProblem={onProblem}
              />
            ))}
          </>
        )}
      </Card>

      <DefaultAlert value={settings.alert_at} onChanged={onChanged} onProblem={onProblem} />

      <Sampling
        sampleFps={settings.sample_fps}
        analyzeFps={settings.fps}
        onChanged={onChanged}
        onProblem={onProblem}
      />

      <Discover known={settings.channels.map((c) => c.login)} onAdd={add} />
    </div>
  );
}

function Row({
  channel,
  entry,
  fallback,
  ramp,
  onChanged,
  onProblem,
}: {
  channel: Channel;
  entry: Live;
  fallback: number;
  ramp: Ramp;
  onChanged: () => void;
  onProblem: (message: string | null) => void;
}) {
  const [draft, setDraft] = useState<number | null>(null);
  const threshold = draft ?? channel.alert_at ?? fallback;
  const percent = entry.current;
  const colour = percent === null ? "var(--color-surface2)" : rampColor(percent, ramp);

  const run = (task: Promise<unknown>) =>
    void task.then(onChanged).catch((err) => onProblem(String(err)));

  return (
    <div
      className={`${COLUMNS} items-center border-b border-surface0 px-5 py-3 last:border-b-0 hover:bg-surface0/25 ${
        channel.enabled ? "" : "opacity-50"
      }`}
    >
      <span className="flex min-w-0 items-center gap-2.5">
        <span
          className="h-2 w-2 flex-none rotate-45 rounded-[1px]"
          style={{ background: colour }}
        />
        <span className="truncate text-[13px] font-medium">{channel.login}</span>
      </span>

      <Status entry={entry} paused={!channel.enabled} />

      <span className="min-w-0">
        <span className="flex items-center gap-2.5">
          <span className="h-1 flex-1 overflow-hidden rounded-full bg-surface0">
            <span
              className="block h-full rounded-full transition-[width] duration-300"
              style={{ width: `${Math.min(percent ?? 0, 100)}%`, background: colour }}
            />
          </span>
          <span
            className="w-[54px] shrink-0 text-right font-mono text-[12px] tabular-nums"
            style={{ color: percent === null ? "var(--color-overlay)" : colour }}
          >
            {percent === null ? "—" : `${percent.toFixed(2)}%`}
          </span>
        </span>
        <span className="mt-1.5 block">
          <History attempts={entry.history} ramp={ramp} height={12} limit={40} />
        </span>
      </span>

      <span className="flex items-center gap-2">
        <Slider
          min={1}
          max={100}
          value={threshold}
          label={`Alert threshold for ${channel.login}`}
          onChange={setDraft}
          onCommit={(value) => {
            // Only after a real drag: every commit restarts the watchers.
            if (draft === null) return;
            setDraft(null);
            if (value !== (channel.alert_at ?? fallback)) {
              run(setThreshold(channel.login, value));
            }
          }}
        />
        <span className="w-[34px] shrink-0 text-right font-mono text-[11px] tabular-nums text-muted">
          {threshold}%
        </span>
      </span>

      <span className="flex justify-end gap-0.5">
        <IconButton
          icon={channel.enabled ? "pause" : "play"}
          label={channel.enabled ? "Pause" : "Resume"}
          onClick={() => run(setEnabled(channel.login, !channel.enabled))}
        />
        <IconButton
          icon="trash"
          label="Remove"
          danger
          onClick={() => run(removeChannel(channel.login))}
        />
      </span>
    </div>
  );
}

const BADGE =
  "inline-flex items-center gap-1.5 rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider";

function Status({ entry, paused }: { entry: Live; paused: boolean }) {
  if (paused) {
    return <span className={`${BADGE} border-surface1 bg-surface0/50 text-overlay`}>paused</span>;
  }

  switch (entry.status.kind) {
    case "watching":
      return (
        <span className={`${BADGE} border-green/40 bg-green/10 text-green`}>
          <span className="h-1.5 w-1.5 rounded-full bg-green" />
          {entry.current === null ? "idle" : "live"}
        </span>
      );
    case "calibrating":
      return <span className={`${BADGE} border-blue/40 bg-blue/10 text-blue`}>scanning</span>;
    case "connecting":
      return <span className={`${BADGE} border-blue/40 bg-blue/10 text-blue`}>opening</span>;
    case "error":
      return (
        <span className={`${BADGE} border-red/40 bg-red/10 text-red`} title={entry.status.message}>
          error
        </span>
      );
    default:
      return (
        <span className={`${BADGE} border-surface1 bg-surface0/50 text-overlay`}>offline</span>
      );
  }
}

/** How fast frames are sampled, trading CPU for lower alert latency. */
function Sampling({
  sampleFps,
  analyzeFps,
  onChanged,
  onProblem,
}: {
  sampleFps: number;
  analyzeFps: number;
  onChanged: () => void;
  onProblem: (message: string | null) => void;
}) {
  const [sample, setSample] = useState<number | null>(null);
  const [analyze, setAnalyze] = useState<number | null>(null);
  const shownSample = sample ?? sampleFps;
  const shownAnalyze = analyze ?? analyzeFps;

  const commit = (nextSample: number, nextAnalyze: number) => {
    setSample(null);
    setAnalyze(null);
    if (nextSample === sampleFps && nextAnalyze === analyzeFps) return;
    void setSampling(nextSample, nextAnalyze)
      .then(onChanged)
      .catch((err) => onProblem(String(err)));
  };

  return (
    <Card title="Performance">
      <div className="grid gap-5 sm:grid-cols-[minmax(0,1fr)_260px]">
        <p className="text-[12.5px] leading-relaxed text-overlay">
          Frames are pulled at the sample rate and checked cheaply; the percentage is read at least
          at the analysis floor, and faster while a run is moving. Raise the sample rate for lower
          alert latency, lower it to spend less CPU.
        </p>
        <div className="flex flex-col gap-4">
          <Field label="Sample rate" value={`${shownSample} fps`}>
            <Slider
              min={Math.max(2, shownAnalyze)}
              max={15}
              value={shownSample}
              label="Frame sample rate"
              onChange={setSample}
              onCommit={(next) => {
                if (sample === null) return;
                commit(next, shownAnalyze);
              }}
            />
          </Field>
          <Field label="Analysis floor" value={`${shownAnalyze} fps`}>
            <Slider
              min={1}
              max={Math.min(8, shownSample)}
              value={shownAnalyze}
              label="OCR analysis floor"
              onChange={setAnalyze}
              onCommit={(next) => {
                if (analyze === null) return;
                commit(shownSample, next);
              }}
            />
          </Field>
        </div>
      </div>
    </Card>
  );
}

/** The threshold every channel without one of its own inherits. */
function DefaultAlert({
  value,
  onChanged,
  onProblem,
}: {
  value: number;
  onChanged: () => void;
  onProblem: (message: string | null) => void;
}) {
  const [draft, setDraft] = useState<number | null>(null);
  const shown = draft ?? value;

  return (
    <Card title="Default alert threshold">
      <div className="grid gap-5 sm:grid-cols-[minmax(0,1fr)_260px]">
        <p className="text-[12.5px] leading-relaxed text-overlay">
          Used by every channel that has not been given a threshold of its own. Start Pos runs never
          alert, however far they get.
        </p>
        <Field label="Alert at" value={`${shown}%`}>
          <Slider
            min={1}
            max={100}
            value={shown}
            label="Default alert threshold"
            onChange={setDraft}
            onCommit={(next) => {
              if (draft === null) return;
              setDraft(null);
              if (next === value) return;
              void setThreshold(null, next)
                .then(onChanged)
                .catch((err) => onProblem(String(err)));
            }}
          />
        </Field>
      </div>
    </Card>
  );
}

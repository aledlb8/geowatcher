import { useCallback, useEffect, useMemo, useState } from "react";
import { getSettings, openConfigFolder, setAppearance } from "../lib/api";
import { type Appearance, BLANK, isLive, type Settings } from "../lib/types";
import { useWatcher } from "../lib/useWatcher";
import { Icon, type IconName } from "../ui/Icon";
import { Button, HEADING } from "../ui/primitives";
import { ChannelsView } from "./ChannelsView";
import { WidgetView } from "./WidgetView";

type Tab = "channels" | "widget";

const ORDER = ["channels", "widget"] as const;

const TABS: Record<Tab, { label: string; icon: IconName; blurb: string }> = {
  channels: {
    label: "Channels",
    icon: "signal",
    blurb: "Who gets watched, and how far a run has to go before you hear about it.",
  },
  widget: {
    label: "Widget",
    icon: "sliders",
    blurb: "The always-on-top card: where it sits, what it shows, how it looks.",
  },
};

export function Configurator() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [tab, setTab] = useState<Tab>("channels");
  const [problem, setProblem] = useState<string | null>(null);
  const live = useWatcher();

  const refresh = useCallback(async () => {
    try {
      setSettings(await getSettings());
      setProblem(null);
    } catch (err) {
      setProblem(String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const watching = useMemo(
    () =>
      settings?.channels.filter((c) => c.enabled && isLive((live[c.login] ?? BLANK).status))
        .length ?? 0,
    [settings, live],
  );

  // Applied as you drag, so the widget updates live rather than hiding behind
  // a save button.
  const applyAppearance = useCallback((next: Appearance) => {
    setSettings((prev) => (prev ? { ...prev, appearance: next } : prev));
    void setAppearance(next).catch((err) => setProblem(String(err)));
  }, []);

  if (!settings) {
    return (
      <div className="grid h-full place-items-center">
        <p className="font-mono text-[12px] text-overlay">{problem ?? "starting the watcher…"}</p>
      </div>
    );
  }

  const active = TABS[tab];
  const total = settings.channels.length;

  return (
    <div className="grid h-full grid-cols-[232px_minmax(0,1fr)]">
      <aside className="flex min-h-0 flex-col border-r border-surface0 bg-mantle">
        <div className="flex items-center gap-2.5 border-b border-surface0 px-5 py-4">
          <span className="h-2.5 w-2.5 rotate-45 rounded-[1px] bg-blue" />
          <span className="text-[14px] font-semibold tracking-tight">GeoWatcher</span>
          <span className="ml-auto font-mono text-[10px] text-overlay">{__APP_VERSION__}</span>
        </div>

        <nav className="p-3">
          <p className={`${HEADING} px-2 pb-2`}>Configure</p>
          {ORDER.map((id) => (
            <button
              key={id}
              type="button"
              aria-current={tab === id ? "page" : undefined}
              onClick={() => setTab(id)}
              className={`relative flex w-full items-center gap-2.5 rounded-md px-2 py-2 text-left text-[13px] transition-colors ${
                tab === id ? "bg-surface0 text-fg" : "text-muted hover:bg-surface0/50 hover:text-fg"
              }`}
            >
              {tab === id && (
                <span className="absolute -left-3 top-1.5 bottom-1.5 w-0.5 rounded-r bg-blue" />
              )}
              <Icon name={TABS[id].icon} size={15} />
              {TABS[id].label}
              {id === "channels" && total > 0 && (
                <span className="ml-auto font-mono text-[10px] tabular-nums text-overlay">
                  {total}
                </span>
              )}
            </button>
          ))}
        </nav>

        <div className="mt-auto border-t border-surface0 p-5">
          <div className="flex items-baseline gap-2">
            <span className={HEADING}>Watching</span>
            <span className="ml-auto font-mono text-[12px] tabular-nums text-fg">
              {watching}/{total}
            </span>
          </div>
          <div className="mt-2 h-0.5 bg-surface0">
            <div
              className="h-full bg-blue transition-all duration-300"
              style={{ width: `${total === 0 ? 0 : (watching / total) * 100}%` }}
            />
          </div>
        </div>
      </aside>

      <div className="flex min-h-0 min-w-0 flex-col">
        <header className="flex items-start gap-6 border-b border-surface0 px-8 py-6">
          <div className="min-w-0">
            <h1 className="text-[22px] font-semibold tracking-tight">{active.label}</h1>
            <p className="mt-1 text-[13px] text-overlay">{active.blurb}</p>
          </div>
          <div className="ml-auto shrink-0">
            <Button variant="ghost" icon="folder" onClick={() => void openConfigFolder()}>
              Config folder
            </Button>
          </div>
        </header>

        <main className="min-h-0 flex-1 overflow-y-auto px-8 py-6">
          {problem && (
            <p className="mb-5 flex items-start gap-2.5 rounded-md border border-red/40 bg-red/10 px-4 py-2.5 text-[12.5px] text-red">
              <Icon name="alert" size={15} />
              {problem}
            </p>
          )}

          {tab === "channels" ? (
            <ChannelsView
              settings={settings}
              live={live}
              onProblem={setProblem}
              onChanged={() => void refresh()}
            />
          ) : (
            <WidgetView appearance={settings.appearance} onChange={applyAppearance} />
          )}
        </main>
      </div>
    </div>
  );
}

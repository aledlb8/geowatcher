// Shapes mirrored from the Rust side. Keep in step with src-tauri/src.

export type Status =
  | { kind: "offline" }
  | { kind: "connecting" }
  | { kind: "calibrating" }
  | { kind: "watching" }
  | { kind: "error"; message: string };

export type Update =
  | { type: "status"; login: string; status: Status }
  | { type: "progress"; login: string; value: number; fromStart: boolean }
  | { type: "alert"; login: string; value: number; streamUrl: string | null }
  | { type: "runEnded"; login: string; peak: number; fromStart: boolean };

export type Channel = {
  login: string;
  alert_at: number | null;
  enabled: boolean;
};

export type Corner = "topLeft" | "topRight" | "bottomLeft" | "bottomRight";

export type Ramp = {
  low: string;
  mid: string;
  high: string;
  peak: string;
};

export type Appearance = {
  corner: Corner;
  marginX: number;
  marginY: number;
  width: number;

  opacity: number;
  radius: number;
  padding: number;
  gap: number;

  fontFamily: string;
  numeralFamily: string;
  fontSize: number;

  background: string;
  panel: string;
  line: string;
  text: string;
  dim: string;
  ramp: Ramp;

  showBar: boolean;
  barHeight: number;
  showHistory: boolean;
  historyLength: number;
  historyHeight: number;
  showOffline: boolean;

  collapseWhenIdle: boolean;
  clickThrough: boolean;
  alwaysOnTop: boolean;
  enabled: boolean;
};

export type Settings = {
  channels: Channel[];
  alert_at: number;
  ignore_start_pos: boolean;
  open_on_click: boolean;
  fps: number;
  sample_fps: number;
  mpv_path: string;
  ffmpeg_path: string;
  appearance: Appearance;
};

/** One finished attempt. */
export type Attempt = {
  /** Monotonic within a session, so the history strip has stable keys. */
  id: number;
  peak: number;
  /** False for Start Pos runs, which are drawn hollow. */
  fromStart: boolean;
};

/** Live per-channel state, accumulated from events. */
export type Live = {
  status: Status;
  current: number | null;
  fromStart: boolean;
  best: number;
  /** Each finished attempt, oldest first. */
  history: Attempt[];
};

export const BLANK: Live = {
  status: { kind: "offline" },
  current: null,
  fromStart: true,
  best: 0,
  history: [],
};

export function isLive(status: Status): boolean {
  return status.kind === "watching" || status.kind === "calibrating";
}

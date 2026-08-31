import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Appearance, Settings, Update } from "./types";

export const getSettings = () => invoke<Settings>("get_settings");
export const addChannel = (login: string) => invoke<boolean>("add_channel", { login });
export const removeChannel = (login: string) => invoke<boolean>("remove_channel", { login });
export const setEnabled = (login: string, enabled: boolean) =>
  invoke<void>("set_enabled", { login, enabled });
export const setThreshold = (login: string | null, percent: number) =>
  invoke<void>("set_threshold", { login, percent });
export const openChannel = (login: string) => invoke<string>("open_channel", { login });
export const discover = (limit?: number) =>
  invoke<Array<{ login: string; viewers: number }>>("discover", { limit });

export const setAppearance = (appearance: Appearance) =>
  invoke<Appearance>("set_appearance", { appearance });
export const resizeWidget = (height: number) => invoke<void>("resize_widget", { height });
export const getUserCss = () => invoke<string>("get_user_css");
export const setUserCss = (css: string) => invoke<void>("set_user_css", { css });
export const openConfigFolder = () => invoke<void>("open_config_folder");

export const onUpdate = (handler: (update: Update) => void) =>
  listen<Update>("watcher://update", ({ payload }) => handler(payload));

export const onHover = (handler: (inside: boolean) => void) =>
  listen<boolean>("widget://hover", ({ payload }) => handler(payload));

export const onUserCss = (handler: (css: string) => void) =>
  listen<string>("widget://css", ({ payload }) => handler(payload));

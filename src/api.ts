import { invoke as tauriInvoke } from "@tauri-apps/api/core";

type Handler = (cmd: string, args?: Record<string, unknown>) => unknown;
let mockHandler: Handler | null = null;
/** Dev-only: route all commands to an in-memory mock (browser preview). */
export const setMockHandler = (h: Handler) => (mockHandler = h);
export const isMocked = () => mockHandler !== null;

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (mockHandler) {
    try {
      return Promise.resolve(mockHandler(cmd, args) as T);
    } catch (e) {
      return Promise.reject(e);
    }
  }
  return tauriInvoke<T>(cmd, args);
}

export type Category = "database" | "website" | "server" | "other";

export interface EntryData {
  title: string;
  category: Category;
  host: string;
  port: number | null;
  username: string;
  password: string;
  notes: string;
  tags: string[];
  favorite: boolean;
}

export interface Entry extends EntryData {
  id: string;
  created_at: number;
  updated_at: number;
}

export interface EntryView extends Omit<EntryData, "password"> {
  id: string;
  has_password: boolean;
  created_at: number;
  updated_at: number;
}

export interface ParsedEntry extends EntryData {
  recognized: string[];
}

export interface AppInfo {
  platform: string;
  window_effect: boolean;
  version: string;
}

export interface Settings {
  clipboard_clear_seconds: number;
}

export type ConflictMode = "skip" | "overwrite" | "keep_both";

export interface ImportReport {
  added: number;
  updated: number;
  skipped: number;
}

export interface ApiError {
  code: string;
  message: string;
}

export const emptyEntry = (): EntryData => ({
  title: "",
  category: "other",
  host: "",
  port: null,
  username: "",
  password: "",
  notes: "",
  tags: [],
  favorite: false,
});

export function errMsg(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as ApiError).message);
  return String(e);
}

export function errCode(e: unknown): string | undefined {
  if (e && typeof e === "object" && "code" in e) return String((e as ApiError).code);
  return undefined;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  list: () => invoke<EntryView[]>("list_entries"),
  get: (id: string) => invoke<Entry>("get_entry", { id }),
  create: (data: EntryData) => invoke<EntryView>("create_entry", { data }),
  createMany: (items: EntryData[]) => invoke<EntryView[]>("create_entries", { items }),
  update: (id: string, data: EntryData) => invoke<EntryView>("update_entry", { id, data }),
  remove: (id: string) => invoke<void>("delete_entry", { id }),
  parse: (text: string) => invoke<ParsedEntry[]>("parse_text", { text }),
  copyField: (id: string, field: "password" | "username" | "host" | "port") =>
    invoke<void>("copy_field", { id, field }),
  copyText: (text: string, sensitive = false) => invoke<void>("copy_text", { text, sensitive }),
  inspectImport: (path: string) => invoke<{ encrypted: boolean }>("inspect_import_file", { path }),
  importFile: (path: string, password: string | null, mode: ConflictMode) =>
    invoke<ImportReport>("import_from_file", { path, password, mode }),
  exportFile: (path: string, password: string | null) => invoke<number>("export_to_file", { path, password }),
};

export const CATEGORY_LABEL: Record<Category, string> = {
  database: "数据库",
  website: "网站",
  server: "服务器",
  other: "其他",
};

export const CATEGORIES: Category[] = ["database", "website", "server", "other"];

export function generatePassword(length = 20): string {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+";
  const buf = new Uint32Array(length);
  crypto.getRandomValues(buf);
  return Array.from(buf, (n) => chars[n % chars.length]).join("");
}

export const isMac = () => navigator.userAgent.includes("Mac");
export const modKey = () => (isMac() ? "⌘" : "Ctrl+");

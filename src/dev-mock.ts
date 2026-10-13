// Dev-only IPC mock so the UI can be previewed in a plain browser (no Tauri).
import { setMockHandler } from "./api";

type Rec = Record<string, unknown> & { id: string; password: string };

export function installMock() {
  const settings = { clipboard_clear_seconds: 0 };
  const entries: Rec[] = [];
  const now = () => Math.floor(Date.now() / 1000);
  const view = (e: Rec) => {
    const { password, ...rest } = e;
    return { ...rest, has_password: !!password };
  };
  const add = (data: Record<string, unknown>) => {
    const e = { ...data, id: crypto.randomUUID(), created_at: now(), updated_at: now() } as unknown as Rec;
    entries.unshift(e);
    return view(e);
  };

  setMockHandler((cmd, args) => {
    const a = (args ?? {}) as Record<string, any>;
    switch (cmd) {
      case "app_info":
        return { platform: "macos", window_effect: false, version: "dev" };
      case "get_settings":
      case "update_settings":
        return Object.assign(settings, a.settings ?? {});
      case "list_entries":
        return entries.map(view);
      case "get_entry":
        return entries.find((e) => e.id === a.id);
      case "create_entry":
        return add(a.data);
      case "create_entries":
        return (a.items as Record<string, unknown>[]).map(add);
      case "update_entry": {
        const e = entries.find((x) => x.id === a.id)!;
        Object.assign(e, a.data, { updated_at: now() });
        return view(e);
      }
      case "delete_entry":
        entries.splice(entries.findIndex((e) => e.id === a.id), 1);
        return;
      case "parse_text":
        return String(a.text)
          .split(/\n\s*\n/)
          .filter((b) => b.trim())
          .map((b) => ({
            title: "腾讯云数据库",
            category: "database",
            host: b.split(/\s+/)[0],
            port: 25729,
            username: "root",
            password: "Demo.456!@#$%^&*()",
            notes: "",
            tags: [],
            favorite: false,
            recognized: ["host", "port", "username", "password"],
          }));
      default:
        return null;
    }
  });
}

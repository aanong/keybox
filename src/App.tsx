import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Layers, Plus, Search, Settings as SettingsIcon, Sparkles, Star, Tag } from "lucide-react";
import { api, AppInfo, CATEGORIES, CATEGORY_LABEL, Category, emptyEntry, EntryData, EntryView, errMsg, modKey } from "./api";
import { QuickAdd } from "./components/QuickAdd";
import { SettingsModal } from "./components/Settings";
import { EntryDetail } from "./components/EntryDetail";
import { CategoryIcon } from "./components/icons";
import { toast, ToastHost } from "./components/Toast";
import { WindowControls } from "./components/WindowControls";

type Filter = { kind: "all" } | { kind: "favorite" } | { kind: "category"; value: Category } | { kind: "tag"; value: string };

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    api.appInfo().then((i) => {
      setInfo(i);
      document.documentElement.dataset.platform = i.platform;
      document.documentElement.classList.toggle("has-effect", i.window_effect);
    });
  }, []);

  if (!info) return null;

  return (
    <>
      {info.platform === "windows" && (
        <div className="win-controls-float">
          <WindowControls />
        </div>
      )}
      <Main />
      <ToastHost />
    </>
  );
}

function Main() {
  const [entries, setEntries] = useState<EntryView[]>([]);
  const [filter, setFilter] = useState<Filter>({ kind: "all" });
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [creating, setCreating] = useState<EntryData | null>(null);
  const [showQuickAdd, setShowQuickAdd] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  const reload = useCallback(async () => {
    try {
      setEntries(await api.list());
    } catch (e) {
      toast(errMsg(e), "error");
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const k = e.key.toLowerCase();
      if (k === "n") {
        e.preventDefault();
        if (e.shiftKey) setCreating(emptyEntry());
        else setShowQuickAdd(true);
      } else if (k === "f") {
        e.preventDefault();
        searchRef.current?.focus();
      } else if (k === ",") {
        e.preventDefault();
        setShowSettings(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const tags = useMemo(() => [...new Set(entries.flatMap((e) => e.tags))].sort(), [entries]);
  const counts = useMemo(() => {
    const c: Record<string, number> = { all: entries.length, favorite: entries.filter((e) => e.favorite).length };
    CATEGORIES.forEach((cat) => (c[cat] = entries.filter((e) => e.category === cat).length));
    return c;
  }, [entries]);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return entries
      .filter((e) => {
        if (filter.kind === "favorite") return e.favorite;
        if (filter.kind === "category") return e.category === filter.value;
        if (filter.kind === "tag") return e.tags.includes(filter.value);
        return true;
      })
      .filter(
        (e) =>
          !q ||
          [e.title, e.host, e.username, e.notes, ...e.tags].some((s) => s.toLowerCase().includes(q)) ||
          String(e.port ?? "").includes(q),
      );
  }, [entries, filter, query]);

  const selected = entries.find((e) => e.id === selectedId) ?? null;
  const filterKey = (f: Filter) => ("value" in f ? `${f.kind}:${f.value}` : f.kind);

  const sideItem = (f: Filter, icon: React.ReactNode, label: string, count?: number) => (
    <button key={filterKey(f)} className={`side-item${filterKey(f) === filterKey(filter) ? " active" : ""}`} onClick={() => setFilter(f)}>
      {icon}
      <span className="side-label">{label}</span>
      {!!count && <span className="side-count">{count}</span>}
    </button>
  );

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="sidebar-top" data-tauri-drag-region />
        <div className="side-section">
          {sideItem({ kind: "all" }, <Layers size={15} />, "全部", counts.all)}
          {sideItem({ kind: "favorite" }, <Star size={15} />, "收藏", counts.favorite)}
        </div>
        <div className="side-heading">分类</div>
        <div className="side-section">
          {CATEGORIES.map((c) => sideItem({ kind: "category", value: c }, <CategoryIcon category={c} size={15} />, CATEGORY_LABEL[c], counts[c]))}
        </div>
        {tags.length > 0 && (
          <>
            <div className="side-heading">标签</div>
            <div className="side-section side-tags">{tags.map((t) => sideItem({ kind: "tag", value: t }, <Tag size={14} />, t))}</div>
          </>
        )}
        <div className="spacer" />
        <div className="side-footer">
          <button className="icon-btn" title={`设置 (${modKey()},)`} onClick={() => setShowSettings(true)}>
            <SettingsIcon size={16} />
          </button>
        </div>
      </aside>

      <section className="list-pane">
        <div className="list-toolbar" data-tauri-drag-region>
          <div className="search">
            <Search size={14} />
            <input ref={searchRef} placeholder="搜索" value={query} onChange={(e) => setQuery(e.target.value)} />
          </div>
          <button className="icon-btn" title={`新建 (${modKey()}⇧N)`} onClick={() => setCreating(emptyEntry())}>
            <Plus size={17} />
          </button>
          <button className="btn btn-primary btn-sm" title={`智能添加 (${modKey()}N)`} onClick={() => setShowQuickAdd(true)}>
            <Sparkles size={14} /> 智能添加
          </button>
        </div>
        <div className="list">
          {visible.length === 0 && (
            <div className="list-empty muted">
              {entries.length === 0 ? (
                <>
                  还没有记录
                  <br />
                  点击「智能添加」粘贴账号信息
                </>
              ) : (
                "没有匹配的记录"
              )}
            </div>
          )}
          {visible.map((e) => (
            <button
              key={e.id}
              className={`list-item${e.id === selectedId && !creating ? " selected" : ""}`}
              onClick={() => {
                setCreating(null);
                setSelectedId(e.id);
              }}
            >
              <div className={`item-icon cat-${e.category}`}>
                <CategoryIcon category={e.category} size={16} />
              </div>
              <div className="item-text">
                <div className="item-title">
                  <span>{e.title}</span>
                  {e.favorite && <Star size={11} className="star-on" />}
                </div>
                <div className="item-sub">{e.username || e.host || "—"}</div>
              </div>
            </button>
          ))}
        </div>
      </section>

      <main className="detail-pane">
        <EntryDetail
          entry={selected}
          creating={creating}
          onCancelCreate={() => setCreating(null)}
          onSaved={async (id) => {
            setCreating(null);
            await reload();
            setSelectedId(id);
          }}
          onDeleted={async () => {
            setSelectedId(null);
            await reload();
          }}
        />
      </main>

      {showQuickAdd && (
        <QuickAdd
          onClose={() => setShowQuickAdd(false)}
          onSaved={async (ids) => {
            setShowQuickAdd(false);
            await reload();
            if (ids.length) setSelectedId(ids[0]);
          }}
        />
      )}
      {showSettings && (
        <SettingsModal onClose={() => setShowSettings(false)} onDataChanged={reload} />
      )}
    </div>
  );
}

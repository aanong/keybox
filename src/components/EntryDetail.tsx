import { useEffect, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Copy, ExternalLink, Eye, EyeOff, Pencil, Star, Trash2 } from "lucide-react";
import { api, CATEGORY_LABEL, EntryData, EntryView, errMsg } from "../api";
import { CategoryIcon } from "./icons";
import { EntryForm } from "./EntryForm";
import { toast } from "./Toast";

interface Props {
  entry: EntryView | null;
  creating: EntryData | null;
  onSaved: (id: string) => void;
  onDeleted: () => void;
  onCancelCreate: () => void;
}

const fmt = (ts: number) => new Date(ts * 1000).toLocaleString("zh-CN", { hour12: false });

export function EntryDetail({ entry, creating, onSaved, onDeleted, onCancelCreate }: Props) {
  const [editing, setEditing] = useState<EntryData | null>(null);
  const [revealed, setRevealed] = useState<string | null>(null);

  useEffect(() => {
    setEditing(null);
    setRevealed(null);
  }, [entry?.id]);

  if (creating) {
    return (
      <EditPane
        title="新建记录"
        initial={creating}
        onCancel={onCancelCreate}
        onSave={async (d) => {
          const v = await api.create(d);
          toast("已添加", "success");
          onSaved(v.id);
        }}
      />
    );
  }

  if (!entry) {
    return (
      <div className="detail empty-detail">
        <p className="muted">选择一条记录查看详情</p>
      </div>
    );
  }

  if (editing) {
    return (
      <EditPane
        key={entry.id}
        title="编辑记录"
        initial={editing}
        onCancel={() => setEditing(null)}
        onSave={async (d) => {
          await api.update(entry.id, d);
          toast("已保存", "success");
          setEditing(null);
          setRevealed(null);
          onSaved(entry.id);
        }}
      />
    );
  }

  const startEdit = async () => {
    try {
      const full = await api.get(entry.id);
      const { id: _id, created_at: _c, updated_at: _u, ...data } = full;
      setEditing(data);
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const copy = async (field: "password" | "username" | "host" | "port", label: string) => {
    try {
      await api.copyField(entry.id, field);
      toast(field === "password" ? `已复制${label}，稍后将自动清除` : `已复制${label}`, "success");
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const toggleReveal = async () => {
    if (revealed !== null) return setRevealed(null);
    try {
      setRevealed((await api.get(entry.id)).password);
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const toggleFavorite = async () => {
    try {
      const full = await api.get(entry.id);
      const { id: _id, created_at: _c, updated_at: _u, ...data } = full;
      await api.update(entry.id, { ...data, favorite: !data.favorite });
      onSaved(entry.id);
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const remove = async () => {
    const ok = await ask(`确定删除「${entry.title}」吗？此操作无法撤销。`, { title: "删除记录", kind: "warning" });
    if (!ok) return;
    try {
      await api.remove(entry.id);
      toast("已删除");
      onDeleted();
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const isUrl = /^https?:\/\//i.test(entry.host);

  return (
    <div className="detail">
      <div className="detail-toolbar" data-tauri-drag-region>
        <button className="icon-btn" title={entry.favorite ? "取消收藏" : "收藏"} onClick={toggleFavorite}>
          <Star size={16} className={entry.favorite ? "star-on" : ""} />
        </button>
        <button className="icon-btn" title="编辑" onClick={startEdit}>
          <Pencil size={16} />
        </button>
        <button className="icon-btn danger" title="删除" onClick={remove}>
          <Trash2 size={16} />
        </button>
      </div>
      <div className="detail-scroll">
        <div className="detail-head">
          <div className={`big-icon cat-${entry.category}`}>
            <CategoryIcon category={entry.category} size={28} />
          </div>
          <div>
            <h2>{entry.title}</h2>
            <div className="muted">{CATEGORY_LABEL[entry.category]}</div>
          </div>
        </div>

        <div className="field-group">
          {entry.host && (
            <Field label={isUrl ? "网址" : "地址"} value={entry.host} mono>
              {isUrl && (
                <button className="icon-btn" title="在浏览器中打开" onClick={() => openUrl(entry.host)}>
                  <ExternalLink size={14} />
                </button>
              )}
              <button className="icon-btn" title="复制" onClick={() => copy("host", "地址")}>
                <Copy size={14} />
              </button>
            </Field>
          )}
          {entry.port != null && (
            <Field label="端口" value={String(entry.port)} mono>
              <button className="icon-btn" title="复制" onClick={() => copy("port", "端口")}>
                <Copy size={14} />
              </button>
            </Field>
          )}
          {entry.username && (
            <Field label="账号" value={entry.username} mono>
              <button className="icon-btn" title="复制" onClick={() => copy("username", "账号")}>
                <Copy size={14} />
              </button>
            </Field>
          )}
          {entry.has_password && (
            <Field label="密码" value={revealed ?? "••••••••••••"} mono secret={revealed === null}>
              <button className="icon-btn" title={revealed === null ? "显示" : "隐藏"} onClick={toggleReveal}>
                {revealed === null ? <Eye size={14} /> : <EyeOff size={14} />}
              </button>
              <button className="icon-btn" title="复制" onClick={() => copy("password", "密码")}>
                <Copy size={14} />
              </button>
            </Field>
          )}
        </div>

        {entry.notes && (
          <div className="field-group">
            <div className="field">
              <div className="field-label">备注</div>
              <div className="field-value notes">{entry.notes}</div>
            </div>
          </div>
        )}

        {entry.tags.length > 0 && (
          <div className="tag-row">
            {entry.tags.map((t) => (
              <span className="tag" key={t}>
                {t}
              </span>
            ))}
          </div>
        )}

        <div className="meta-line muted">
          创建于 {fmt(entry.created_at)} · 修改于 {fmt(entry.updated_at)}
        </div>
      </div>
    </div>
  );
}

function Field(props: { label: string; value: string; mono?: boolean; secret?: boolean; children?: React.ReactNode }) {
  return (
    <div className="field">
      <div className="field-label">{props.label}</div>
      <div className={`field-value${props.mono ? " mono" : ""}${props.secret ? " secret" : ""}`}>{props.value}</div>
      <div className="field-actions">{props.children}</div>
    </div>
  );
}

function EditPane(props: { title: string; initial: EntryData; onCancel: () => void; onSave: (d: EntryData) => Promise<void> }) {
  const [data, setData] = useState(props.initial);
  const [saving, setSaving] = useState(false);
  const save = async () => {
    setSaving(true);
    try {
      await props.onSave(data);
    } catch (e) {
      toast(errMsg(e), "error");
    } finally {
      setSaving(false);
    }
  };
  return (
    <div className="detail">
      <div className="detail-toolbar" data-tauri-drag-region>
        <span className="toolbar-title">{props.title}</span>
        <div className="spacer" />
        <button className="btn" onClick={props.onCancel}>
          取消
        </button>
        <button className="btn btn-primary" onClick={save} disabled={saving}>
          保存
        </button>
      </div>
      <div className="detail-scroll">
        <EntryForm value={data} onChange={setData} />
      </div>
    </div>
  );
}

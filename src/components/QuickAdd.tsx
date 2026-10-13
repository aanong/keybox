import { useEffect, useRef, useState } from "react";
import { ClipboardPaste, Sparkles, Trash2 } from "lucide-react";
import { api, emptyEntry, EntryData, errMsg, ParsedEntry } from "../api";
import { EntryForm } from "./EntryForm";
import { Modal } from "./Modal";
import { toast } from "./Toast";

interface Props {
  onClose: () => void;
  onSaved: (ids: string[]) => void;
}

interface Item {
  key: number;
  data: EntryData;
  recognized: string[];
}

let keySeq = 0;
const toItem = (p: ParsedEntry): Item => {
  const { recognized, ...data } = p;
  return { key: ++keySeq, data, recognized };
};

export function QuickAdd({ onClose, onSaved }: Props) {
  const [text, setText] = useState("");
  const [items, setItems] = useState<Item[]>([]);
  const [saving, setSaving] = useState(false);
  const textRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textRef.current?.focus();
  }, []);

  useEffect(() => {
    const h = setTimeout(async () => {
      if (!text.trim()) return setItems([]);
      try {
        setItems((await api.parse(text)).map(toItem));
      } catch (e) {
        toast(errMsg(e), "error");
      }
    }, 200);
    return () => clearTimeout(h);
  }, [text]);

  const save = async () => {
    const list = items.map((i) => i.data);
    if (!list.length) return;
    setSaving(true);
    try {
      const created = await api.createMany(list);
      toast(`已添加 ${created.length} 条记录`, "success");
      onSaved(created.map((c) => c.id));
    } catch (e) {
      toast(errMsg(e), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title="智能添加"
      onClose={onClose}
      width={680}
      footer={
        <>
          <button className="btn" onClick={() => setItems([...items, { key: ++keySeq, data: emptyEntry(), recognized: [] }])}>
            手动添加一条
          </button>
          <div className="spacer" />
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn btn-primary" disabled={!items.length || saving} onClick={save}>
            {items.length > 1 ? `全部添加（${items.length}）` : "添加"}
          </button>
        </>
      }
    >
      <div className="paste-box">
        <div className="paste-hint">
          <ClipboardPaste size={14} /> 粘贴包含网址、端口、账号、密码的文本，自动识别。多条记录用空行分隔。
        </div>
        <textarea
          ref={textRef}
          className="input mono"
          rows={4}
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={"例如：\ngz-cdb-xxx.sql.tencentcdb.com  port:25729  user root  password Abc!@#\n网址：https://mail.qq.com 账号：张三 密码：我的密码"}
        />
      </div>
      {items.length > 0 && (
        <div className="parsed-list">
          {items.map((it, idx) => (
            <div className="parsed-card" key={it.key}>
              <div className="parsed-card-head">
                <Sparkles size={13} />
                <span>
                  记录 {idx + 1}
                  {it.recognized.length > 0 && <em> · 已识别 {it.recognized.length} 个字段</em>}
                </span>
                <div className="spacer" />
                <button className="icon-btn" title="移除" onClick={() => setItems(items.filter((x) => x.key !== it.key))}>
                  <Trash2 size={14} />
                </button>
              </div>
              <EntryForm
                compact
                value={it.data}
                highlight={it.recognized}
                onChange={(d) => setItems(items.map((x) => (x.key === it.key ? { ...x, data: d } : x)))}
              />
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}

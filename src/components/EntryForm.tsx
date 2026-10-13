import { useState } from "react";
import { Eye, EyeOff, Star, Wand2 } from "lucide-react";
import { CATEGORIES, CATEGORY_LABEL, Category, EntryData, generatePassword } from "../api";

interface Props {
  value: EntryData;
  onChange: (v: EntryData) => void;
  compact?: boolean;
  highlight?: string[];
}

export function EntryForm({ value, onChange, compact, highlight = [] }: Props) {
  const [show, setShow] = useState(false);
  const [tagText, setTagText] = useState(value.tags.join(", "));
  const set = <K extends keyof EntryData>(k: K, v: EntryData[K]) => onChange({ ...value, [k]: v });
  const hl = (f: string) => (highlight.includes(f) ? " field-recognized" : "");

  return (
    <div className={`entry-form${compact ? " compact" : ""}`}>
      <div className="form-row two">
        <label>
          <span>标题</span>
          <input className="input" value={value.title} onChange={(e) => set("title", e.target.value)} placeholder="例如：腾讯云数据库" />
        </label>
        <label>
          <span>分类</span>
          <select className="input" value={value.category} onChange={(e) => set("category", e.target.value as Category)}>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {CATEGORY_LABEL[c]}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="form-row host-port">
        <label>
          <span>地址 / 网址</span>
          <input className={`input mono${hl("host")}`} value={value.host} onChange={(e) => set("host", e.target.value)} placeholder="example.com 或 https://…" />
        </label>
        <label>
          <span>端口</span>
          <input
            className={`input mono${hl("port")}`}
            inputMode="numeric"
            value={value.port ?? ""}
            onChange={(e) => {
              const n = e.target.value.replace(/\D/g, "");
              set("port", n ? Math.min(65535, Number(n)) : null);
            }}
          />
        </label>
      </div>
      <div className="form-row two">
        <label>
          <span>账号</span>
          <input className={`input mono${hl("username")}`} value={value.username} onChange={(e) => set("username", e.target.value)} />
        </label>
        <label>
          <span>密码</span>
          <div className="input-group">
            <input
              className={`input mono${hl("password")}`}
              type={show ? "text" : "password"}
              value={value.password}
              onChange={(e) => set("password", e.target.value)}
              autoComplete="off"
            />
            <button type="button" className="icon-btn" title={show ? "隐藏" : "显示"} onClick={() => setShow(!show)}>
              {show ? <EyeOff size={15} /> : <Eye size={15} />}
            </button>
            <button
              type="button"
              className="icon-btn"
              title="生成强密码"
              onClick={() => {
                set("password", generatePassword());
                setShow(true);
              }}
            >
              <Wand2 size={15} />
            </button>
          </div>
        </label>
      </div>
      {!compact && (
        <div className="form-row">
          <label>
            <span>标签（逗号分隔）</span>
            <input
              className="input"
              value={tagText}
              onChange={(e) => {
                setTagText(e.target.value);
                set(
                  "tags",
                  e.target.value
                    .split(/[,，]/)
                    .map((t) => t.trim())
                    .filter(Boolean),
                );
              }}
              placeholder="生产, 公司"
            />
          </label>
        </div>
      )}
      <div className="form-row">
        <label>
          <span>备注</span>
          <textarea className="input" rows={compact ? 2 : 4} value={value.notes} onChange={(e) => set("notes", e.target.value)} />
        </label>
      </div>
      {!compact && (
        <label className="checkbox">
          <input type="checkbox" checked={value.favorite} onChange={(e) => set("favorite", e.target.checked)} />
          <Star size={14} /> 收藏
        </label>
      )}
    </div>
  );
}

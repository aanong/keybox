import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Download, Upload } from "lucide-react";
import { api, ConflictMode, errCode, errMsg, Settings as SettingsT } from "../api";
import { Modal } from "./Modal";
import { toast } from "./Toast";

interface Props {
  onClose: () => void;
  onDataChanged: () => void;
}

const CLIP_OPTIONS = [10, 30, 60, 120, 0];

export function SettingsModal({ onClose, onDataChanged }: Props) {
  const [settings, setSettings] = useState<SettingsT | null>(null);
  const [tab, setTab] = useState<"general" | "data">("general");

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => toast(errMsg(e), "error"));
  }, []);

  const update = async (patch: Partial<SettingsT>) => {
    if (!settings) return;
    try {
      setSettings(await api.updateSettings({ ...settings, ...patch }));
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  return (
    <Modal title="设置" onClose={onClose} width={560}>
      <div className="segmented">
        <button className={tab === "general" ? "active" : ""} onClick={() => setTab("general")}>
          通用
        </button>
        <button className={tab === "data" ? "active" : ""} onClick={() => setTab("data")}>
          导入 / 导出
        </button>
      </div>

      {tab === "general" && settings && (
        <>
          <div className="settings-group">
            <div className="settings-row">
              <span>复制密码后清除剪贴板</span>
              <select
                className="input small"
                value={settings.clipboard_clear_seconds}
                onChange={(e) => update({ clipboard_clear_seconds: Number(e.target.value) })}
              >
                {CLIP_OPTIONS.map((s) => (
                  <option key={s} value={s}>
                    {s === 0 ? "从不" : `${s} 秒后`}
                  </option>
                ))}
              </select>
            </div>
          </div>
          <div className="settings-group muted">
            数据以明文保存在本机：{dataPath()}
          </div>
        </>
      )}

      {tab === "data" && <DataTab onDataChanged={onDataChanged} />}
    </Modal>
  );
}

function dataPath() {
  return navigator.userAgent.includes("Windows")
    ? "%APPDATA%\\com.keybox.app\\keybox.db"
    : "~/Library/Application Support/com.keybox.app/keybox.db";
}

function DataTab({ onDataChanged }: { onDataChanged: () => void }) {
  const [encrypt, setEncrypt] = useState(false);
  const [exportPw, setExportPw] = useState("");
  const [importPath, setImportPath] = useState<string | null>(null);
  const [importEncrypted, setImportEncrypted] = useState(false);
  const [importPw, setImportPw] = useState("");
  const [mode, setMode] = useState<ConflictMode>("skip");
  const [busy, setBusy] = useState(false);

  const doExport = async () => {
    if (encrypt && exportPw.length < 6) return toast("导出密码至少需要 6 个字符", "error");
    const date = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `keybox-${date}${encrypt ? "-encrypted" : ""}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    setBusy(true);
    try {
      const n = await api.exportFile(path, encrypt ? exportPw : null);
      toast(`已导出 ${n} 条记录`, "success");
      setExportPw("");
    } catch (e) {
      toast(errMsg(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const pickImport = async () => {
    const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path || Array.isArray(path)) return;
    try {
      const info = await api.inspectImport(path);
      setImportPath(path);
      setImportEncrypted(info.encrypted);
      setImportPw("");
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const doImport = async () => {
    if (!importPath) return;
    setBusy(true);
    try {
      const r = await api.importFile(importPath, importEncrypted ? importPw : null, mode);
      toast(`导入完成：新增 ${r.added}，更新 ${r.updated}，跳过 ${r.skipped}`, "success");
      setImportPath(null);
      onDataChanged();
    } catch (e) {
      toast(errCode(e) === "NEED_PASSWORD" ? "请输入文件的导出密码" : errMsg(e), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="settings-group">
        <div className="settings-title">
          <Upload size={14} /> 导出为 JSON
        </div>
        <label className="checkbox">
          <input type="checkbox" checked={encrypt} onChange={(e) => setEncrypt(e.target.checked)} />
          加密导出（设置导出密码）
        </label>
        {encrypt ? (
          <input
            className="input"
            type="password"
            placeholder="设置导出密码（导入时需要）"
            value={exportPw}
            onChange={(e) => setExportPw(e.target.value)}
          />
        ) : (
          <div className="warn-text">明文导出的文件包含所有密码，请妥善保管。</div>
        )}
        <button className="btn btn-primary" onClick={doExport} disabled={busy}>
          导出…
        </button>
      </div>

      <div className="settings-group">
        <div className="settings-title">
          <Download size={14} /> 从 JSON 导入
        </div>
        <button className="btn" onClick={pickImport} disabled={busy}>
          选择文件…
        </button>
        {importPath && (
          <>
            <div className="muted file-path">{importPath}</div>
            {importEncrypted && (
              <input
                className="input"
                type="password"
                placeholder="此文件已加密，请输入导出密码"
                value={importPw}
                onChange={(e) => setImportPw(e.target.value)}
              />
            )}
            <div className="settings-row">
              <span>遇到重复记录（地址 + 账号相同）</span>
              <select className="input small" value={mode} onChange={(e) => setMode(e.target.value as ConflictMode)}>
                <option value="skip">跳过</option>
                <option value="overwrite">覆盖</option>
                <option value="keep_both">保留两者</option>
              </select>
            </div>
            <button className="btn btn-primary" onClick={doImport} disabled={busy || (importEncrypted && !importPw)}>
              开始导入
            </button>
          </>
        )}
      </div>
    </>
  );
}

import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mac-style traffic lights for frameless windows (Windows platform). */
export function WindowControls() {
  const w = getCurrentWindow();
  return (
    <div className="traffic-lights">
      <button className="tl tl-close" onClick={() => w.close()} title="关闭" />
      <button className="tl tl-min" onClick={() => w.minimize()} title="最小化" />
      <button className="tl tl-max" onClick={() => w.toggleMaximize()} title="最大化" />
    </div>
  );
}

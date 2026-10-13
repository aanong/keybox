import { useEffect, useState } from "react";

type Kind = "info" | "error" | "success";
interface Toast {
  id: number;
  text: string;
  kind: Kind;
}

let listeners: ((t: Toast[]) => void)[] = [];
let toasts: Toast[] = [];
let seq = 0;

function emit() {
  listeners.forEach((l) => l(toasts));
}

export function toast(text: string, kind: Kind = "info") {
  const t = { id: ++seq, text, kind };
  toasts = [...toasts, t];
  emit();
  setTimeout(() => {
    toasts = toasts.filter((x) => x.id !== t.id);
    emit();
  }, kind === "error" ? 4000 : 2200);
}

export function ToastHost() {
  const [items, setItems] = useState<Toast[]>(toasts);
  useEffect(() => {
    listeners.push(setItems);
    return () => {
      listeners = listeners.filter((l) => l !== setItems);
    };
  }, []);
  return (
    <div className="toast-host">
      {items.map((t) => (
        <div key={t.id} className={`toast toast-${t.kind}`}>
          {t.text}
        </div>
      ))}
    </div>
  );
}

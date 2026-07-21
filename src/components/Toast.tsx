import { useState, useEffect, useCallback, createContext, useContext, useRef } from "react";

interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
  visible: boolean;
}

interface ToastContextValue {
  toast: (message: string, type?: "success" | "error" | "info") => void;
}

const ToastContext = createContext<ToastContextValue>({ toast: () => {} });

export function useToast() {
  return useContext(ToastContext);
}

let toastId = 0;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timersRef = useRef<Map<number, number>>(new Map());

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.map((t) => (t.id === id ? { ...t, visible: false } : t)));
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 300);
  }, []);

  const toast = useCallback((message: string, type: "success" | "error" | "info" = "info") => {
    const id = ++toastId;
    setToasts((prev) => [...prev.slice(-4), { id, message, type, visible: true }]);
    const timer = window.setTimeout(() => removeToast(id), 3000);
    timersRef.current.set(id, timer);
  }, [removeToast]);

  useEffect(() => {
    return () => {
      timersRef.current.forEach((t) => clearTimeout(t));
    };
  }, []);

  return (
    <ToastContext.Provider value={{ toast }}>
      {children}
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={`pointer-events-auto px-4 py-2.5 rounded-lg shadow-lg text-xs font-medium backdrop-blur-sm transition-all duration-300 transform ${
              t.visible ? "translate-x-0 opacity-100" : "translate-x-full opacity-0"
            } ${
              t.type === "success"
                ? "bg-green-900/90 text-green-200 border border-green-700/50"
                : t.type === "error"
                ? "bg-red-900/90 text-red-200 border border-red-700/50"
                : "bg-[#1a2236]/90 text-gray-200 border border-[#1e293b]/50"
            }`}
          >
            <div className="flex items-center gap-2">
              <span className="shrink-0">
                {t.type === "success" ? "✓" : t.type === "error" ? "✕" : "ℹ"}
              </span>
              <span>{t.message}</span>
              <button
                onClick={() => removeToast(t.id)}
                className="ml-2 text-gray-500 hover:text-gray-300 shrink-0"
              >
                ×
              </button>
            </div>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

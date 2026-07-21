import { useState, useEffect, useRef, useCallback } from "react";

export interface ContextMenuItem {
  label: string;
  icon?: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}

interface ContextMenuState {
  x: number;
  y: number;
  items: ContextMenuItem[];
}

const ContextMenuContext = {
  state: null as ContextMenuState | null,
  set: null as ((state: ContextMenuState | null) => void) | null,
};

export function useContextMenu() {
  const [, rerender] = useState(0);

  useEffect(() => {
    const handler = () => {
      if (ContextMenuContext.set) ContextMenuContext.set(null);
    };
    window.addEventListener("click", handler);
    window.addEventListener("contextmenu", handler);
    return () => {
      window.removeEventListener("click", handler);
      window.removeEventListener("contextmenu", handler);
    };
  }, []);

  const show = useCallback((e: React.MouseEvent, items: ContextMenuItem[]) => {
    e.preventDefault();
    e.stopPropagation();
    ContextMenuContext.set?.({ x: e.clientX, y: e.clientY, items });
    rerender((n) => n + 1);
  }, []);

  return { showContextMenu: show };
}

export function ContextMenuRenderer() {
  const [state, setState] = useState<ContextMenuState | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  ContextMenuContext.state = state;
  ContextMenuContext.set = setState;

  useEffect(() => {
    if (!state || !menuRef.current) return;
    const menu = menuRef.current;
    const rect = menu.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    if (rect.right > vw) menu.style.left = `${state.x - rect.width}px`;
    if (rect.bottom > vh) menu.style.top = `${state.y - rect.height}px`;
  }, [state]);

  if (!state) return null;

  return (
    <div
      ref={menuRef}
      className="fixed z-[100] min-w-[160px] py-1 rounded-lg shadow-xl border border-[#1e293b] backdrop-blur-sm"
      style={{
        left: state.x,
        top: state.y,
        background: "rgba(13, 17, 23, 0.95)",
      }}
      onClick={(e) => e.stopPropagation()}
    >
      {state.items.map((item, i) => (
        <button
          key={i}
          disabled={item.disabled}
          onClick={() => {
            item.onClick();
            setState(null);
          }}
          className={`w-full text-left px-3 py-1.5 text-xs flex items-center gap-2 transition-colors ${
            item.disabled
              ? "text-gray-700 cursor-not-allowed"
              : item.danger
              ? "text-red-400 hover:bg-red-900/30"
              : "text-gray-300 hover:bg-[#1a2236] hover:text-white"
          }`}
        >
          {item.icon && <span className="w-4 text-center text-[10px]">{item.icon}</span>}
          {item.label}
        </button>
      ))}
    </div>
  );
}

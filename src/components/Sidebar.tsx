import type { ViewMode } from "../types";
import { APP_VERSION } from "../constants";

interface SidebarProps {
  currentView: ViewMode;
  onNavigate: (view: ViewMode) => void;
  packetCount: number;
  savedCount: number;
  sequenceCount: number;
}

const navItems: { id: ViewMode; label: string; icon: string }[] = [
  { id: "capture", label: "Capture", icon: "◉" },
  { id: "library", label: "Library", icon: "📦" },
  { id: "replay", label: "Replay", icon: "▶" },
  { id: "sequences", label: "Sequences", icon: "⛓" },
  { id: "fuzzer", label: "Fuzzer", icon: "⚡" },
  { id: "stats", label: "Statistics", icon: "📊" },
];

export function Sidebar({
  currentView,
  onNavigate,
  packetCount,
  savedCount,
  sequenceCount,
}: SidebarProps) {
  const badges: Record<ViewMode, number> = {
    capture: packetCount,
    library: savedCount,
    replay: 0,
    sequences: sequenceCount,
    fuzzer: 0,
    stats: 0,
  };

  return (
    <aside className="w-[220px] bg-[#0d1117] border-r border-[#1e293b] flex flex-col shrink-0">
      <div className="p-4 border-b border-[#1e293b]">
        <div className="flex items-center gap-2">
          <img src="/logo-mark.svg" alt="byteplay logo" className="w-8 h-8" draggable={false} />
          <div>
            <h1 className="text-sm font-semibold text-white">byteplay</h1>
            <p className="text-[10px] text-gray-500">Packet Analyzer</p>
          </div>
        </div>
      </div>

      <nav className="flex-1 p-2 space-y-0.5">
        {navItems.map((item) => {
          const isActive = currentView === item.id;
          const badge = badges[item.id];
          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors ${
                isActive
                  ? "bg-blue-600/20 text-blue-400 border border-blue-600/30"
                  : "text-gray-400 hover:bg-[#161b22] hover:text-gray-200 border border-transparent"
              }`}
            >
              <span className="text-base w-5 text-center">{item.icon}</span>
              <span className="flex-1 text-left">{item.label}</span>
              {badge > 0 && (
                <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-gray-800 text-gray-400">
                  {badge > 999 ? `${(badge / 1000).toFixed(1)}k` : badge}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="p-3 border-t border-[#1e293b]">
        <div className="text-[10px] text-gray-600 text-center">v{APP_VERSION} • Local-first</div>
        <div className="text-[9px] text-gray-700 text-center mt-1">For authorized testing only</div>
      </div>
    </aside>
  );
}

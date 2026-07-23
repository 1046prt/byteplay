import { useState, useEffect, useMemo } from "react";
import { commands } from "../commands";
import type { CaptureStatsData } from "../types";

const PROTOCOL_COLORS: Record<string, string> = {
  TCP: "#3b82f6",
  UDP: "#22c55e",
  ICMP: "#eab308",
  ICMPv6: "#eab308",
  Other: "#6b7280",
};

export function StatsView({ packetCount }: { packetCount: number }) {
  const [stats, setStats] = useState<CaptureStatsData | null>(null);
  const [loading, setLoading] = useState(false);

  const loadStats = async () => {
    setLoading(true);
    try {
      const data = await commands.getCaptureStats();
      setStats(data);
    } catch (e) {
      console.error("Failed to load stats:", e);
    }
    setLoading(false);
  };

  useEffect(() => {
    loadStats();
  }, [packetCount]);

  const maxTimelineCount = useMemo(
    () => Math.max(1, ...((stats?.timeline ?? []).map((t) => t.count))),
    [stats]
  );

  if (!stats) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-sm">
        {loading ? "Loading stats..." : "No capture data"}
      </div>
    );
  }

  return (
    <div className="h-full overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-white">Capture Statistics</h2>
        <button onClick={loadStats} className="btn btn-secondary text-xs">
          Refresh
        </button>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-4 gap-3">
        <StatCard label="Total Packets" value={stats.total_packets.toLocaleString()} />
        <StatCard label="Total Bytes" value={formatBytes(stats.total_bytes)} />
        <StatCard
          label="Protocols"
          value={stats.protocols.length.toString()}
        />
        <StatCard
          label="Unique Sources"
          value={stats.top_sources.length.toString()}
        />
      </div>

      <div className="grid grid-cols-2 gap-6">
        {/* Protocol breakdown */}
        <div className="panel p-4">
          <h3 className="text-sm font-semibold text-white mb-3">Protocol Breakdown</h3>
          <div className="space-y-2">
            {stats.protocols.map((p) => {
              const pct = stats.total_packets > 0 ? (p.count / stats.total_packets) * 100 : 0;
              const color = PROTOCOL_COLORS[p.protocol] || PROTOCOL_COLORS.Other;
              return (
                <div key={p.protocol}>
                  <div className="flex items-center justify-between text-xs mb-1">
                    <span className="text-gray-300 font-medium">{p.protocol}</span>
                    <span className="text-gray-500">
                      {p.count.toLocaleString()} ({pct.toFixed(1)}%) — {formatBytes(p.bytes)}
                    </span>
                  </div>
                  <div className="h-2 bg-[#111827] rounded-full overflow-hidden">
                    <div
                      className="h-full rounded-full transition-all"
                      style={{ width: `${pct}%`, backgroundColor: color }}
                    />
                  </div>
                </div>
              );
            })}
            {stats.protocols.length === 0 && (
              <p className="text-xs text-gray-600">No packets captured yet</p>
            )}
          </div>
        </div>

        {/* Traffic timeline */}
        <div className="panel p-4">
          <h3 className="text-sm font-semibold text-white mb-3">Traffic Timeline</h3>
          <div className="flex items-end gap-1 h-[140px]">
            {stats.timeline.map((t, i) => {
              const h = maxTimelineCount > 0 ? (t.count / maxTimelineCount) * 120 : 0;
              return (
                <div key={i} className="flex-1 flex flex-col items-center justify-end h-full group relative">
                  <div className="absolute bottom-full mb-1 hidden group-hover:block z-10 bg-[#1e293b] border border-[#2d3748] rounded px-2 py-1 text-[10px] text-gray-300 whitespace-nowrap pointer-events-none">
                    {t.timestamp}: {t.count} pkts ({formatBytes(t.bytes)})
                  </div>
                  <div
                    className="w-full bg-blue-500/70 rounded-t min-h-[1px] hover:bg-blue-400 transition-colors cursor-default"
                    style={{ height: `${h}px` }}
                  />
                  {i % Math.max(1, Math.floor(stats.timeline.length / 8)) === 0 && (
                    <span className="text-[9px] text-gray-600 mt-1 rotate-[-30deg] origin-top-left whitespace-nowrap">
                      {t.timestamp}
                    </span>
                  )}
                </div>
              );
            })}
            {stats.timeline.length === 0 && (
              <p className="text-xs text-gray-600 mx-auto">No timeline data</p>
            )}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-6">
        {/* Top sources */}
        <div className="panel p-4">
          <h3 className="text-sm font-semibold text-white mb-3">Top Sources</h3>
          <div className="space-y-1">
            {stats.top_sources.slice(0, 10).map((ep, i) => {
              const pct = stats.total_packets > 0 ? (ep.count / stats.total_packets) * 100 : 0;
              return (
                <div key={ep.endpoint} className="flex items-center gap-2 text-xs">
                  <span className="text-gray-600 w-4 text-right">{i + 1}.</span>
                  <span className="text-gray-300 font-mono flex-1 truncate">{ep.endpoint}</span>
                  <div className="w-20 h-1.5 bg-[#111827] rounded-full overflow-hidden">
                    <div className="h-full bg-green-500/60 rounded-full" style={{ width: `${pct}%` }} />
                  </div>
                  <span className="text-gray-500 w-16 text-right">{ep.count.toLocaleString()}</span>
                </div>
              );
            })}
            {stats.top_sources.length === 0 && (
              <p className="text-xs text-gray-600">No data</p>
            )}
          </div>
        </div>

        {/* Top destinations */}
        <div className="panel p-4">
          <h3 className="text-sm font-semibold text-white mb-3">Top Destinations</h3>
          <div className="space-y-1">
            {stats.top_destinations.slice(0, 10).map((ep, i) => {
              const pct = stats.total_packets > 0 ? (ep.count / stats.total_packets) * 100 : 0;
              return (
                <div key={ep.endpoint} className="flex items-center gap-2 text-xs">
                  <span className="text-gray-600 w-4 text-right">{i + 1}.</span>
                  <span className="text-gray-300 font-mono flex-1 truncate">{ep.endpoint}</span>
                  <div className="w-20 h-1.5 bg-[#111827] rounded-full overflow-hidden">
                    <div className="h-full bg-blue-500/60 rounded-full" style={{ width: `${pct}%` }} />
                  </div>
                  <span className="text-gray-500 w-16 text-right">{ep.count.toLocaleString()}</span>
                </div>
              );
            })}
            {stats.top_destinations.length === 0 && (
              <p className="text-xs text-gray-600">No data</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="panel p-3 flex flex-col">
      <span className="text-[10px] text-gray-500 uppercase tracking-wider">{label}</span>
      <span className="text-xl font-bold text-white mt-1">{value}</span>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

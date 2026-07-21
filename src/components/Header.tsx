import type { CaptureStats } from "../types";

interface HeaderProps {
  isCapturing: boolean;
  packetCount: number;
  statusMessage: string;
  stats: CaptureStats;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds.toFixed(0)}s`;
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}m ${s}s`;
}

export function Header({ isCapturing, packetCount, statusMessage, stats }: HeaderProps) {
  return (
    <header className="h-10 bg-[#0d1117] border-b border-[#1e293b] flex items-center px-4 gap-4 shrink-0">
      <div className="flex items-center gap-2">
        {isCapturing && (
          <span className="relative flex h-2.5 w-2.5">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75" />
            <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-red-500" />
          </span>
        )}
        <span className="text-xs text-gray-400">
          {isCapturing ? "Capturing" : "Idle"}
        </span>
      </div>
      <div className="h-4 w-px bg-[#1e293b]" />
      <span className="text-xs text-gray-500">
        {packetCount.toLocaleString()} pkts
      </span>
      {isCapturing && (
        <>
          <div className="h-4 w-px bg-[#1e293b]" />
          <span className="text-[11px] text-cyan-400 font-mono">
            {stats.packetsPerSecond.toLocaleString()} pps
          </span>
          <div className="h-4 w-px bg-[#1e293b]" />
          <span className="text-[11px] text-cyan-400 font-mono">
            {formatBytes(stats.bytesPerSecond)}/s
          </span>
          <div className="h-4 w-px bg-[#1e293b]" />
          <span className="text-[11px] text-cyan-400 font-mono">
            {formatDuration(stats.duration)}
          </span>
          <div className="h-4 w-px bg-[#1e293b]" />
          <span className="text-[11px] text-gray-500 font-mono">
            {formatBytes(stats.totalBytes)} total
          </span>
        </>
      )}
      <div className="flex-1" />
      <span className="text-xs text-gray-500 font-mono">{statusMessage}</span>
    </header>
  );
}

import { useRef, useCallback, useEffect, useState, useMemo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { HEX_DISPLAY_LIMIT } from "../constants";
import type { CapturedPacket } from "../types";

interface PacketListProps {
  packets: CapturedPacket[];
  selectedPacket: CapturedPacket | null;
  onSelectPacket: (p: CapturedPacket) => void;
  onContextMenu?: (e: React.MouseEvent, p: CapturedPacket) => void;
}

type SortKey = "index" | "time" | "protocol" | "src" | "dst" | "length";
type SortDir = "asc" | "desc";

function formatTime(ts: string): string {
  try {
    const d = new Date(ts);
    return (
      d.toLocaleTimeString("en-US", { hour12: false }) +
      "." +
      String(d.getMilliseconds()).padStart(3, "0")
    );
  } catch {
    return ts;
  }
}

function getProtocol(p: CapturedPacket): string {
  if (p.tcp) return "TCP";
  if (p.udp) return "UDP";
  if (p.ipv4?.protocol) return p.ipv4.protocol;
  if (p.ipv6?.next_header) return p.ipv6.next_header;
  return "—";
}

function getSrcEndpoint(p: CapturedPacket): string {
  const ip = p.ipv4?.src_ip || p.ipv6?.src_ip || "—";
  const port = p.tcp?.src_port ?? p.udp?.src_port;
  return port !== undefined ? `${ip}:${port}` : ip;
}

function getDstEndpoint(p: CapturedPacket): string {
  const ip = p.ipv4?.dst_ip || p.ipv6?.dst_ip || "—";
  const port = p.tcp?.dst_port ?? p.udp?.dst_port;
  return port !== undefined ? `${ip}:${port}` : ip;
}

function getProtocolColor(protocol: string): string {
  switch (protocol) {
    case "TCP":
      return "text-blue-400";
    case "UDP":
      return "text-green-400";
    case "ICMP":
      return "text-yellow-400";
    case "ICMPv6":
      return "text-yellow-400";
    default:
      return "text-gray-400";
  }
}

function getFlags(p: CapturedPacket): string {
  if (p.tcp?.flags) {
    const f = p.tcp.flags;
    const parts: string[] = [];
    if (f.syn) parts.push("SYN");
    if (f.ack) parts.push("ACK");
    if (f.fin) parts.push("FIN");
    if (f.rst) parts.push("RST");
    if (f.psh) parts.push("PSH");
    if (f.urg) parts.push("URG");
    return parts.join("|");
  }
  return "";
}

function getPreview(p: CapturedPacket): string {
  const ascii = p.payload_ascii;
  if (ascii.length > HEX_DISPLAY_LIMIT) return ascii.substring(0, HEX_DISPLAY_LIMIT) + "…";
  return ascii || "—";
}

interface ColumnDef {
  key: SortKey;
  label: string;
  width: string;
  align?: "right";
}

const COLUMNS: ColumnDef[] = [
  { key: "index", label: "#", width: "w-[42px]" },
  { key: "time", label: "Time", width: "w-[110px]" },
  { key: "protocol", label: "Proto", width: "w-[50px]", align: "right" },
  { key: "src", label: "Source", width: "flex-1 min-w-0" },
  { key: "dst", label: "Destination", width: "flex-1 min-w-0" },
  { key: "length", label: "Len", width: "w-[55px]", align: "right" },
];

function SortIcon({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) return <span className="text-gray-700 ml-0.5">↕</span>;
  return <span className="text-blue-400 ml-0.5">{dir === "asc" ? "↑" : "↓"}</span>;
}

export function PacketList({
  packets,
  selectedPacket,
  onSelectPacket,
  onContextMenu,
}: PacketListProps) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [sortKey, setSortKey] = useState<SortKey>("index");
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const sortedPackets = useMemo(() => {
    const sorted = [...packets];
    sorted.sort((a, b) => {
      let cmp = 0;
      switch (sortKey) {
        case "index":
          cmp = a.capture_index - b.capture_index;
          break;
        case "time":
          cmp = a.timestamp.localeCompare(b.timestamp);
          break;
        case "protocol":
          cmp = getProtocol(a).localeCompare(getProtocol(b));
          break;
        case "src":
          cmp = getSrcEndpoint(a).localeCompare(getSrcEndpoint(b));
          break;
        case "dst":
          cmp = getDstEndpoint(a).localeCompare(getDstEndpoint(b));
          break;
        case "length":
          cmp = a.frame_length - b.frame_length;
          break;
      }
      return sortDir === "asc" ? cmp : -cmp;
    });
    return sorted;
  }, [packets, sortKey, sortDir]);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  };

  const virtualizer = useVirtualizer({
    count: sortedPackets.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 32,
    overscan: 20,
  });

  const scrollToIndex = useCallback(
    (index: number) => {
      virtualizer.scrollToIndex(index, { align: "auto" });
    },
    [virtualizer]
  );

  useEffect(() => {
    if (selectedPacket) {
      const idx = sortedPackets.findIndex((p) => p.id === selectedPacket.id);
      if (idx >= 0) scrollToIndex(idx);
    }
  }, [selectedPacket, sortedPackets, scrollToIndex]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center px-3 py-1 bg-[#0b0f19] border-b border-[#1e293b] text-[10px] font-semibold text-gray-500 uppercase tracking-wider shrink-0 select-none">
        {COLUMNS.map((col) => (
          <button
            key={col.key}
            onClick={() => handleSort(col.key)}
            className={`flex items-center shrink-0 hover:text-gray-300 transition-colors ${col.width} ${col.align === "right" ? "text-right justify-end" : ""}`}
          >
            {col.label}
            <SortIcon active={sortKey === col.key} dir={sortDir} />
          </button>
        ))}
        <span className="w-[70px] shrink-0 text-right">Flags</span>
        <span className="w-[200px] shrink-0 ml-2">Payload</span>
      </div>
      <div ref={parentRef} className="flex-1 overflow-auto" style={{ contain: "strict" }}>
        <div
          style={{
            height: `${virtualizer.getTotalSize()}px`,
            width: "100%",
            position: "relative",
          }}
        >
          {virtualizer.getVirtualItems().map((virtualRow) => {
            const p = sortedPackets[virtualRow.index];
            const isSelected = selectedPacket?.id === p.id;
            const protocol = getProtocol(p);
            const hasTcpFlags = p.tcp?.flags;
            const isSyn = hasTcpFlags && p.tcp!.flags.syn && !p.tcp!.flags.ack;
            const isRst = hasTcpFlags && p.tcp!.flags.rst;

            return (
              <div
                key={p.id}
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
                onClick={() => onSelectPacket(p)}
                onContextMenu={(e) => onContextMenu?.(e, p)}
                className={`absolute w-full flex items-center px-3 py-1 text-xs font-mono cursor-pointer border-b border-[#111827] transition-colors ${
                  isSelected
                    ? "bg-blue-600/20 border-l-2 border-l-blue-500"
                    : "hover:bg-[#111827] border-l-2 border-l-transparent"
                } ${isRst ? "bg-red-900/10" : isSyn ? "bg-green-900/10" : ""}`}
                style={{
                  top: `${virtualRow.start}px`,
                  height: `${virtualRow.size}px`,
                }}
              >
                <span className="w-[42px] text-gray-600 text-[10px] shrink-0">
                  {p.capture_index}
                </span>
                <span className="w-[110px] text-gray-500 shrink-0">{formatTime(p.timestamp)}</span>
                <span
                  className={`w-[50px] text-center font-semibold shrink-0 ${getProtocolColor(protocol)}`}
                >
                  {protocol}
                </span>
                <span className="flex-1 truncate text-gray-300 min-w-0">{getSrcEndpoint(p)}</span>
                <span className="text-gray-600 mx-1 shrink-0">→</span>
                <span className="flex-1 truncate text-gray-300 min-w-0">{getDstEndpoint(p)}</span>
                <span className="w-[70px] text-[10px] text-gray-500 shrink-0 truncate text-right pr-1">
                  {getFlags(p) || ""}
                </span>
                <span className="w-[55px] text-right text-gray-500 shrink-0">{p.frame_length}</span>
                <span className="w-[200px] truncate text-gray-600 ml-2 shrink-0">
                  {getPreview(p)}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

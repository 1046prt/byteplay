import { useRef, useCallback, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { HEX_DISPLAY_LIMIT } from "../constants";
import type { CapturedPacket } from "../types";

interface PacketListProps {
  packets: CapturedPacket[];
  selectedPacket: CapturedPacket | null;
  onSelectPacket: (p: CapturedPacket) => void;
}

function formatTime(ts: string): string {
  try {
    const d = new Date(ts);
    return d.toLocaleTimeString("en-US", { hour12: false }) +
      "." +
      String(d.getMilliseconds()).padStart(3, "0");
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
    case "TCP": return "text-blue-400";
    case "UDP": return "text-green-400";
    case "ICMP": return "text-yellow-400";
    case "ICMPv6": return "text-yellow-400";
    default: return "text-gray-400";
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

export function PacketList({ packets, selectedPacket, onSelectPacket }: PacketListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: packets.length,
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
      const idx = packets.findIndex((p) => p.id === selectedPacket.id);
      if (idx >= 0) scrollToIndex(idx);
    }
  }, [selectedPacket, packets, scrollToIndex]);

  return (
    <div
      ref={parentRef}
      className="h-full overflow-auto"
      style={{ contain: "strict" }}
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const p = packets[virtualRow.index];
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
              <span className="w-[28px] text-gray-600 text-[10px] shrink-0">
                {p.capture_index}
              </span>
              <span className="w-[110px] text-gray-500 shrink-0">
                {formatTime(p.timestamp)}
              </span>
              <span className={`w-[40px] text-center font-semibold shrink-0 ${getProtocolColor(protocol)}`}>
                {protocol}
              </span>
              <span className="flex-1 truncate text-gray-300 min-w-0">
                {getSrcEndpoint(p)}
              </span>
              <span className="text-gray-600 mx-1 shrink-0">→</span>
              <span className="flex-1 truncate text-gray-300 min-w-0">
                {getDstEndpoint(p)}
              </span>
              {hasTcpFlags && (
                <span className="w-[70px] text-[10px] text-gray-500 shrink-0 truncate">
                  {getFlags(p)}
                </span>
              )}
              <span className="w-[50px] text-right text-gray-500 shrink-0">
                {p.frame_length}
              </span>
              <span className="w-[200px] truncate text-gray-600 ml-2 shrink-0">
                {getPreview(p)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

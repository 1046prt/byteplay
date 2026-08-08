import { useState, useEffect, useCallback } from "react";
import { commands } from "../commands";
import type { CapturedPacket, HexDiffEntry } from "../types";
import { HexViewer } from "./HexViewer";

interface PacketDetailProps {
  packet: CapturedPacket | null;
}

type DetailTab = "headers" | "payload" | "raw" | "diff";

export function PacketDetail({ packet }: PacketDetailProps) {
  const [tab, setTab] = useState<DetailTab>("headers");
  const [editablePayload, setEditablePayload] = useState<string>("");
  const [isEditing, setIsEditing] = useState(false);
  const [modifiedBytes, setModifiedBytes] = useState<number[] | null>(null);
  const [diffEntries, setDiffEntries] = useState<HexDiffEntry[] | null>(null);

  useEffect(() => {
    setTab("headers");
    setIsEditing(false);
    setModifiedBytes(null);
    setDiffEntries(null);
  }, [packet?.id]);

  const handleApplyEdit = useCallback(() => {
    if (!packet) return;
    const newBytes = Array.from(new TextEncoder().encode(editablePayload));
    setModifiedBytes(newBytes);
    setIsEditing(false);
    setTab("diff");
    commands.computeHexDiff(packet.payload, newBytes).then(setDiffEntries);
  }, [packet, editablePayload]);

  const handleCancelEdit = useCallback(() => {
    setIsEditing(false);
    if (packet) setEditablePayload(packet.payload_ascii);
  }, [packet]);

  if (!packet) {
    return (
      <div className="h-full flex items-center justify-center text-gray-600 text-sm">
        Select a packet to view details
      </div>
    );
  }

  const protocol = packet.tcp ? "TCP" : packet.udp ? "UDP" : packet.ipv4?.protocol || "—";

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center gap-1 p-2 bg-[#0d1117] border-b border-[#1e293b]">
        {(["headers", "payload", "raw", "diff"] as DetailTab[]).map((t) => (
          <button
            key={t}
            onClick={() => {
              setTab(t);
              if (t === "payload" && packet) {
                setEditablePayload(packet.payload_ascii);
                setIsEditing(false);
              }
            }}
            className={`px-3 py-1 text-xs rounded-md capitalize transition-colors ${
              tab === t ? "bg-blue-600/20 text-blue-400" : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {t === "diff" ? (
              <span className="flex items-center gap-1">
                diff
                {diffEntries && <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />}
              </span>
            ) : (
              t
            )}
          </button>
        ))}
        <div className="flex-1" />
        <span className="text-[10px] text-gray-600 font-mono">
          {protocol} • {packet.frame_length} bytes
        </span>
      </div>

      <div className="flex-1 overflow-auto p-3">
        {tab === "headers" && <HeadersView packet={packet} />}
        {tab === "payload" && (
          <PayloadView
            packet={packet}
            editablePayload={editablePayload}
            setEditablePayload={setEditablePayload}
            isEditing={isEditing}
            setIsEditing={setIsEditing}
            onApply={handleApplyEdit}
            onCancel={handleCancelEdit}
          />
        )}
        {tab === "raw" && <HexViewer bytes={packet.raw_bytes} />}
        {tab === "diff" && (
          <DiffView
            original={packet.payload}
            modified={modifiedBytes}
            entries={diffEntries}
            originalLabel="Original payload"
            modifiedLabel="Modified payload"
          />
        )}
      </div>
    </div>
  );
}

function HeadersView({ packet }: { packet: CapturedPacket }) {
  return (
    <div className="space-y-3">
      {packet.ethernet && (
        <HeaderSection title="Ethernet">
          <HeaderField label="Source MAC" value={packet.ethernet.src_mac} />
          <HeaderField label="Dest MAC" value={packet.ethernet.dst_mac} />
          <HeaderField label="Type" value={packet.ethernet.ether_type} />
        </HeaderSection>
      )}

      {packet.ipv4 && (
        <HeaderSection title="IPv4">
          <HeaderField label="Source" value={packet.ipv4.src_ip} />
          <HeaderField label="Destination" value={packet.ipv4.dst_ip} />
          <HeaderField label="Version" value={String(packet.ipv4.version)} />
          <HeaderField label="IHL" value={`${packet.ipv4.ihl} (${packet.ipv4.ihl * 4} bytes)`} />
          <HeaderField label="TTL" value={String(packet.ipv4.ttl)} />
          <HeaderField label="Protocol" value={packet.ipv4.protocol} />
          <HeaderField label="Total Length" value={String(packet.ipv4.total_length)} />
          <HeaderField
            label="Identification"
            value={`0x${packet.ipv4.identification.toString(16)}`}
          />
          <HeaderField label="Flags" value={`0x${packet.ipv4.flags.toString(16)}`} />
          <HeaderField label="Checksum" value={`0x${packet.ipv4.checksum.toString(16)}`} />
        </HeaderSection>
      )}

      {packet.ipv6 && (
        <HeaderSection title="IPv6">
          <HeaderField label="Source" value={packet.ipv6.src_ip} />
          <HeaderField label="Destination" value={packet.ipv6.dst_ip} />
          <HeaderField label="Version" value={String(packet.ipv6.version)} />
          <HeaderField label="Traffic Class" value={String(packet.ipv6.traffic_class)} />
          <HeaderField label="Hop Limit" value={String(packet.ipv6.hop_limit)} />
          <HeaderField label="Next Header" value={packet.ipv6.next_header} />
          <HeaderField label="Payload Length" value={String(packet.ipv6.payload_length)} />
        </HeaderSection>
      )}

      {packet.tcp && (
        <HeaderSection title="TCP">
          <HeaderField label="Source Port" value={String(packet.tcp.src_port)} />
          <HeaderField label="Dest Port" value={String(packet.tcp.dst_port)} />
          <HeaderField label="Sequence" value={String(packet.tcp.sequence)} />
          <HeaderField label="Ack Number" value={String(packet.tcp.ack_number)} />
          <HeaderField
            label="Data Offset"
            value={`${packet.tcp.data_offset} (${packet.tcp.data_offset * 4} bytes)`}
          />
          <HeaderField label="Window" value={String(packet.tcp.window)} />
          <HeaderField label="Checksum" value={`0x${packet.tcp.checksum.toString(16)}`} />
          <HeaderField label="Flags" value={formatTcpFlags(packet.tcp.flags)} />
        </HeaderSection>
      )}

      {packet.udp && (
        <HeaderSection title="UDP">
          <HeaderField label="Source Port" value={String(packet.udp.src_port)} />
          <HeaderField label="Dest Port" value={String(packet.udp.dst_port)} />
          <HeaderField label="Length" value={String(packet.udp.length)} />
          <HeaderField label="Checksum" value={`0x${packet.udp.checksum.toString(16)}`} />
        </HeaderSection>
      )}

      {packet.payload.length > 0 && (
        <HeaderSection title="Payload">
          <div className="text-xs text-gray-400 font-mono break-all">{packet.payload_hex}</div>
          <div className="text-xs text-gray-500 font-mono mt-1 whitespace-pre-wrap break-all">
            {packet.payload_ascii}
          </div>
        </HeaderSection>
      )}
    </div>
  );
}

function HeaderSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="panel p-3">
      <h4 className="text-xs font-semibold text-gray-300 mb-2 uppercase tracking-wide">{title}</h4>
      <div className="space-y-1">{children}</div>
    </div>
  );
}

function HeaderField({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center text-xs">
      <span className="w-[120px] text-gray-500 shrink-0">{label}</span>
      <span className="font-mono text-gray-300 truncate">{value}</span>
    </div>
  );
}

function formatTcpFlags(flags: {
  syn: boolean;
  ack: boolean;
  fin: boolean;
  rst: boolean;
  psh: boolean;
  urg: boolean;
}): string {
  const parts: string[] = [];
  if (flags.syn) parts.push("SYN");
  if (flags.ack) parts.push("ACK");
  if (flags.fin) parts.push("FIN");
  if (flags.rst) parts.push("RST");
  if (flags.psh) parts.push("PSH");
  if (flags.urg) parts.push("URG");
  return parts.length > 0 ? parts.join(" | ") : "None";
}

function PayloadView({
  packet,
  editablePayload,
  setEditablePayload,
  isEditing,
  setIsEditing,
  onApply,
  onCancel,
}: {
  packet: CapturedPacket;
  editablePayload: string;
  setEditablePayload: (v: string) => void;
  isEditing: boolean;
  setIsEditing: (v: boolean) => void;
  onApply: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <button
          onClick={() => {
            if (isEditing) {
              onCancel();
            } else {
              setIsEditing(true);
            }
          }}
          className={`btn text-xs ${isEditing ? "btn-secondary" : "btn-secondary"}`}
        >
          {isEditing ? "Cancel" : "Edit Payload"}
        </button>
        {isEditing && (
          <button onClick={onApply} className="btn btn-primary text-xs">
            Apply & Diff
          </button>
        )}
        {isEditing && (
          <span className="text-[10px] text-gray-500">
            Modify ASCII below, then apply to see hex diff
          </span>
        )}
      </div>
      <div className="panel p-3">
        {isEditing ? (
          <textarea
            value={editablePayload}
            onChange={(e) => setEditablePayload(e.target.value)}
            className="w-full h-[300px] bg-transparent font-mono text-xs text-gray-300 resize-none outline-none"
            spellCheck={false}
            autoFocus
          />
        ) : (
          <div className="font-mono text-xs text-gray-300 whitespace-pre-wrap break-all min-h-[300px]">
            {packet.payload_ascii || "(empty payload)"}
          </div>
        )}
      </div>
      {packet.payload.length > 0 && (
        <div className="panel p-3">
          <h4 className="text-xs font-semibold text-gray-400 mb-2">Hex</h4>
          <HexViewer bytes={packet.payload} />
        </div>
      )}
    </div>
  );
}

function DiffView({
  original: _original,
  modified,
  entries,
  originalLabel,
  modifiedLabel,
}: {
  original: number[];
  modified: number[] | null;
  entries: HexDiffEntry[] | null;
  originalLabel: string;
  modifiedLabel: string;
}) {
  if (!modified || !entries) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-sm">
        Edit the payload in the Payload tab to see a diff
      </div>
    );
  }

  const changedCount = entries.filter((e) => e.changed).length;
  const BYTES_PER_LINE = 16;

  const lines: Array<{
    offset: number;
    origBytes: Array<{ val: number; changed: boolean }>;
    modBytes: Array<{ val: number; changed: boolean }>;
    origAscii: string;
    modAscii: string;
  }> = [];

  for (let i = 0; i < entries.length; i += BYTES_PER_LINE) {
    const origBytes: Array<{ val: number; changed: boolean }> = [];
    const modBytes: Array<{ val: number; changed: boolean }> = [];
    let origAscii = "";
    let modAscii = "";

    for (let j = 0; j < BYTES_PER_LINE && i + j < entries.length; j++) {
      const entry = entries[i + j];
      const ob = entry.original ?? 0;
      const mb = entry.modified ?? 0;

      origBytes.push({ val: ob, changed: entry.changed });
      modBytes.push({ val: mb, changed: entry.changed });

      origAscii += entry.changed ? "·" : ob >= 0x20 && ob <= 0x7e ? String.fromCharCode(ob) : ".";
      modAscii += entry.changed ? (mb >= 0x20 && mb <= 0x7e ? String.fromCharCode(mb) : "·") : "·";
    }
    lines.push({ offset: i, origBytes, modBytes, origAscii, modAscii });
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-4 text-xs">
        <span className="text-gray-400">{originalLabel}</span>
        <span className="text-gray-600">→</span>
        <span className="text-gray-400">{modifiedLabel}</span>
        <div className="flex-1" />
        <span className="text-amber-400 font-mono">
          {changedCount} byte{changedCount !== 1 ? "s" : ""} changed
        </span>
      </div>

      <div className="panel p-3 font-mono text-[11px] leading-5">
        <div className="flex text-gray-600 mb-1 text-[10px] select-none">
          <span className="w-[70px] shrink-0">Offset</span>
          <span className="flex gap-0">
            {Array.from({ length: BYTES_PER_LINE }, (_, i) => (
              <span key={i} className="w-[22px] text-center">
                {i.toString(16).padStart(2, "0").toUpperCase()}
              </span>
            ))}
          </span>
          <span className="ml-2 w-[130px] shrink-0">ASCII</span>
        </div>

        {lines.map((line) => (
          <div key={line.offset} className="flex">
            <span className="w-[70px] text-gray-600 shrink-0">
              {line.offset.toString(16).padStart(8, "0")}
            </span>
            <span className="flex gap-0">
              {line.origBytes.map((entry, j) => (
                <span
                  key={j}
                  className={`w-[22px] text-center ${
                    entry.changed ? "bg-red-900/40 text-red-400 font-bold" : "text-gray-400"
                  }`}
                >
                  {entry.val.toString(16).padStart(2, "0")}
                </span>
              ))}
              {line.origBytes.length < BYTES_PER_LINE &&
                Array.from({ length: BYTES_PER_LINE - line.origBytes.length }, (_, i) => (
                  <span key={`pad-${i}`} className="w-[22px] text-center text-gray-800">
                    ·
                  </span>
                ))}
            </span>
            <span className="ml-2 w-[130px] text-gray-500 shrink-0 whitespace-pre">
              {line.origAscii}
            </span>
          </div>
        ))}

        <div className="border-t border-[#1e293b] my-1" />

        {lines.map((line) => (
          <div key={`mod-${line.offset}`} className="flex">
            <span className="w-[70px] text-gray-600 shrink-0">
              {line.offset.toString(16).padStart(8, "0")}
            </span>
            <span className="flex gap-0">
              {line.modBytes.map((entry, j) => (
                <span
                  key={j}
                  className={`w-[22px] text-center ${
                    entry.changed ? "bg-green-900/40 text-green-400 font-bold" : "text-gray-500"
                  }`}
                >
                  {entry.val.toString(16).padStart(2, "0")}
                </span>
              ))}
              {line.modBytes.length < BYTES_PER_LINE &&
                Array.from({ length: BYTES_PER_LINE - line.modBytes.length }, (_, i) => (
                  <span key={`pad-${i}`} className="w-[22px] text-center text-gray-800">
                    ·
                  </span>
                ))}
            </span>
            <span className="ml-2 w-[130px] text-gray-500 shrink-0 whitespace-pre">
              {line.modAscii}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

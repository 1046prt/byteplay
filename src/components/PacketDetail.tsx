import { useState } from "react";
import type { CapturedPacket } from "../types";
import { HexViewer } from "./HexViewer";
import { commands } from "../commands";

interface PacketDetailProps {
  packet: CapturedPacket | null;
  onSaved: () => void;
  setStatusMessage: (msg: string) => void;
}

type DetailTab = "headers" | "payload" | "raw";

export function PacketDetail({ packet, onSaved, setStatusMessage }: PacketDetailProps) {
  const [tab, setTab] = useState<DetailTab>("headers");
  const [editablePayload, setEditablePayload] = useState<string>("");
  const [isEditing, setIsEditing] = useState(false);

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
        {(["headers", "payload", "raw"] as DetailTab[]).map((t) => (
          <button
            key={t}
            onClick={() => {
              setTab(t);
              if (t === "payload" && packet) {
                setEditablePayload(packet.payload_ascii);
              }
              setIsEditing(false);
            }}
            className={`px-3 py-1 text-xs rounded-md capitalize transition-colors ${
              tab === t
                ? "bg-blue-600/20 text-blue-400"
                : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {t}
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
            setStatusMessage={setStatusMessage}
          />
        )}
        {tab === "raw" && (
          <HexViewer
            bytes={packet.raw_bytes}
            editable={false}
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
          <HeaderField label="Identification" value={`0x${packet.ipv4.identification.toString(16)}`} />
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
          <HeaderField label="Data Offset" value={`${packet.tcp.data_offset} (${packet.tcp.data_offset * 4} bytes)`} />
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
          <div className="text-xs text-gray-400 font-mono break-all">
            {packet.payload_hex}
          </div>
          <div className="text-xs text-gray-500 font-mono mt-1 whitespace-pre-wrap break-all">
            {packet.payload_ascii}
          </div>
        </HeaderSection>
      )}
    </div>
  );
}

function HeaderSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="panel p-3">
      <h4 className="text-xs font-semibold text-gray-300 mb-2 uppercase tracking-wide">
        {title}
      </h4>
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
  setStatusMessage,
}: {
  packet: CapturedPacket;
  editablePayload: string;
  setEditablePayload: (v: string) => void;
  isEditing: boolean;
  setIsEditing: (v: boolean) => void;
  setStatusMessage: (msg: string) => void;
}) {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <button
          onClick={() => setIsEditing(!isEditing)}
          className={`btn text-xs ${isEditing ? "btn-primary" : "btn-secondary"}`}
        >
          {isEditing ? "Editing" : "Edit Payload"}
        </button>
        {isEditing && (
          <span className="text-[10px] text-gray-500">
            Modify the ASCII payload below
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
          <HexViewer bytes={packet.payload} editable={false} />
        </div>
      )}
    </div>
  );
}

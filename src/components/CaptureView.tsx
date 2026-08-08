import { useState, useEffect, useMemo, useRef, useCallback } from "react";
import { commands } from "../commands";
import { useToast } from "./Toast";
import { useContextMenu, type ContextMenuItem } from "./ContextMenu";
import type { CapturedPacket, CaptureConfig } from "../types";
import { PacketList } from "./PacketList";
import { PacketDetail } from "./PacketDetail";

interface CaptureViewProps {
  packets: CapturedPacket[];
  selectedPacket: CapturedPacket | null;
  onSelectPacket: (p: CapturedPacket | null) => void;
  isCapturing: boolean;
  setIsCapturing: (v: boolean) => void;
  onClearPackets: () => void;
  onSaved: () => void;
  onImportPcap: (imported: CapturedPacket[]) => void;
  setStatusMessage: (msg: string) => void;
}

export function CaptureView({
  packets,
  selectedPacket,
  onSelectPacket,
  isCapturing,
  setIsCapturing,
  onClearPackets,
  onSaved,
  onImportPcap,
  setStatusMessage,
}: CaptureViewProps) {
  const [interfaces, setInterfaces] = useState<string[]>([]);
  const [selectedInterface, setSelectedInterface] = useState("");
  const [bpfFilter, setBpfFilter] = useState("");
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [filterText, setFilterText] = useState("");
  const [debouncedFilter, setDebouncedFilter] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);
  const [protocolFilter, setProtocolFilter] = useState<string>("all");
  const [isDragging, setIsDragging] = useState(false);
  const debounceRef = useRef<number | null>(null);
  const { toast } = useToast();
  const { showContextMenu } = useContextMenu();

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => setDebouncedFilter(filterText), 150);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [filterText]);

  useEffect(() => {
    const handler = () => setShowSaveDialog(true);
    window.addEventListener("packetforge:save-packet", handler);
    return () => window.removeEventListener("packetforge:save-packet", handler);
  }, []);

  const loadInterfaces = useCallback(async () => {
    try {
      const ifaces = await commands.listInterfaces();
      setInterfaces(ifaces);
      if (ifaces.length > 0 && !selectedInterface) {
        setSelectedInterface(ifaces[0]);
      }
    } catch (e) {
      setStatusMessage(`Failed to list interfaces: ${e}`);
    }
  }, [selectedInterface, setStatusMessage]);

  useEffect(() => {
    loadInterfaces();
  }, [loadInterfaces]);

  const handleStartCapture = async () => {
    if (!selectedInterface) {
      toast("No interface selected", "error");
      return;
    }
    try {
      const config: CaptureConfig = {
        interface_name: selectedInterface.split(" [")[0],
        bpf_filter: bpfFilter,
        max_packets: null,
      };
      await commands.startCapture(config);
      setIsCapturing(true);
      toast(`Capturing on ${config.interface_name}`, "success");
    } catch (e) {
      toast(`Failed to start capture: ${e}`, "error");
    }
  };

  const handleStopCapture = async () => {
    try {
      await commands.stopCapture();
      setIsCapturing(false);
      toast(`Capture stopped — ${packets.length} packets`, "info");
    } catch (e) {
      toast(`Failed to stop capture: ${e}`, "error");
    }
  };

  const handleSavePacket = async (name: string, description: string, tags: string[]) => {
    if (!selectedPacket) return;
    try {
      await commands.savePacket(selectedPacket.id, name, description, tags);
      toast(`Packet saved as "${name}"`, "success");
      setShowSaveDialog(false);
      onSaved();
    } catch (e) {
      toast(`Failed to save packet: ${e}`, "error");
    }
  };

  const handleClear = () => {
    if (confirmClear) {
      onClearPackets();
      setConfirmClear(false);
      toast("Packets cleared", "info");
    } else {
      setConfirmClear(true);
      setTimeout(() => setConfirmClear(false), 3000);
    }
  };

  const handleImportFile = async (file: File) => {
    if (!file.name.endsWith(".pcap") && !file.name.endsWith(".pcapng")) {
      toast("Only .pcap files are supported", "error");
      return;
    }
    try {
      toast(`Importing ${file.name}...`, "info");
      const imported = await commands.importPcap(file.name);
      onImportPcap(imported);
      toast(`Imported ${imported.length} packets from ${file.name}`, "success");
    } catch (e) {
      toast(`Import failed: ${e}`, "error");
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => setIsDragging(false);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const file = e.dataTransfer.files[0];
    if (file) handleImportFile(file);
  };

  const handlePacketContextMenu = (e: React.MouseEvent, p: CapturedPacket) => {
    const items: ContextMenuItem[] = [
      {
        label: "Save packet",
        icon: "💾",
        onClick: () => {
          onSelectPacket(p);
          setShowSaveDialog(true);
        },
      },
      {
        label: "Replay packet",
        icon: "▶",
        onClick: () => {
          onSelectPacket(p);
          setStatusMessage("Switch to Replay view to configure");
        },
      },
      {
        label: "Copy source IP",
        icon: "📋",
        onClick: () => {
          const ip = p.ipv4?.src_ip || p.ipv6?.src_ip || "";
          navigator.clipboard.writeText(ip);
          toast(`Copied: ${ip}`, "success");
        },
      },
      {
        label: "Copy dest IP",
        icon: "📋",
        onClick: () => {
          const ip = p.ipv4?.dst_ip || p.ipv6?.dst_ip || "";
          navigator.clipboard.writeText(ip);
          toast(`Copied: ${ip}`, "success");
        },
      },
      {
        label: "Copy payload hex",
        icon: "📋",
        onClick: () => {
          navigator.clipboard.writeText(p.payload_hex);
          toast("Payload hex copied", "success");
        },
      },
      {
        label: "Copy full hex",
        icon: "📋",
        onClick: () => {
          navigator.clipboard.writeText(
            p.raw_bytes.map((b) => b.toString(16).padStart(2, "0")).join(" ")
          );
          toast("Full hex copied", "success");
        },
      },
    ];
    showContextMenu(e, items);
  };

  const filteredPackets = useMemo(
    () =>
      packets.filter((p) => {
        if (protocolFilter !== "all") {
          const proto = p.tcp ? "tcp" : p.udp ? "udp" : "other";
          if (proto !== protocolFilter) return false;
        }
        if (!debouncedFilter) return true;
        const lower = debouncedFilter.toLowerCase();
        return (
          p.payload_hex.toLowerCase().includes(lower) ||
          p.payload_ascii.toLowerCase().includes(lower) ||
          p.ipv4?.src_ip.includes(lower) ||
          p.ipv4?.dst_ip.includes(lower) ||
          p.ipv6?.src_ip?.includes(lower) ||
          p.ipv6?.dst_ip?.includes(lower) ||
          p.tcp?.src_port.toString().includes(lower) ||
          p.tcp?.dst_port.toString().includes(lower) ||
          p.udp?.src_port.toString().includes(lower) ||
          p.udp?.dst_port.toString().includes(lower)
        );
      }),
    [packets, debouncedFilter, protocolFilter]
  );

  return (
    <div
      className="flex h-full relative"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {isDragging && (
        <div className="absolute inset-0 z-50 bg-blue-600/20 border-2 border-dashed border-blue-400 flex items-center justify-center pointer-events-none">
          <div className="text-center">
            <div className="text-3xl mb-2">📂</div>
            <p className="text-sm text-blue-300 font-medium">Drop .pcap file to import</p>
          </div>
        </div>
      )}
      <div className="flex flex-col flex-1 min-w-0 border-r border-[#1e293b]">
        <div className="flex items-center gap-2 p-2 bg-[#0d1117] border-b border-[#1e293b]">
          <select
            value={selectedInterface}
            onChange={(e) => setSelectedInterface(e.target.value)}
            className="input input-mono text-xs max-w-[300px]"
          >
            {interfaces.map((iface) => (
              <option key={iface} value={iface}>
                {iface}
              </option>
            ))}
          </select>
          <input
            type="text"
            placeholder="BPF filter (e.g. port 80)"
            value={bpfFilter}
            onChange={(e) => setBpfFilter(e.target.value)}
            className="input input-mono text-xs flex-1 max-w-[250px]"
          />
          {!isCapturing ? (
            <button onClick={handleStartCapture} className="btn btn-success text-xs">
              ● Capture
            </button>
          ) : (
            <button onClick={handleStopCapture} className="btn btn-danger text-xs">
              ■ Stop
            </button>
          )}
          <div className="h-5 w-px bg-[#1e293b] mx-1" />
          <div className="relative flex items-center">
            <input
              type="text"
              placeholder="Filter..."
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
              className="input text-xs w-[180px] pr-6"
            />
            {filterText && (
              <button
                onClick={() => setFilterText("")}
                className="absolute right-1.5 text-gray-600 hover:text-gray-300 text-[10px]"
              >
                ✕
              </button>
            )}
          </div>
          {debouncedFilter && (
            <span className="text-[10px] text-gray-500 font-mono shrink-0">
              {filteredPackets.length}/{packets.length}
            </span>
          )}
          <button
            onClick={handleClear}
            className={`btn text-xs ${confirmClear ? "btn-danger" : "btn-secondary"}`}
          >
            {confirmClear ? "Confirm?" : "Clear"}
          </button>
          {selectedPacket && (
            <button onClick={() => setShowSaveDialog(true)} className="btn btn-primary text-xs">
              Save
            </button>
          )}
          {packets.length > 0 && (
            <button onClick={() => setShowExportDialog(true)} className="btn btn-secondary text-xs">
              Export
            </button>
          )}
          <label className="btn btn-secondary text-xs cursor-pointer">
            Import
            <input
              type="file"
              accept=".pcap,.pcapng"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) handleImportFile(file);
                e.target.value = "";
              }}
            />
          </label>
          <div className="h-5 w-px bg-[#1e293b] mx-1" />
          <div className="flex items-center gap-0.5">
            {(["all", "tcp", "udp", "other"] as const).map((proto) => (
              <button
                key={proto}
                onClick={() => setProtocolFilter(proto)}
                className={`px-2 py-0.5 text-[10px] font-medium rounded transition-colors ${
                  protocolFilter === proto
                    ? "bg-blue-600/30 text-blue-400"
                    : "text-gray-500 hover:text-gray-300 hover:bg-[#161b22]"
                }`}
              >
                {proto.toUpperCase()}
              </button>
            ))}
          </div>
        </div>
        <PacketList
          packets={filteredPackets}
          selectedPacket={selectedPacket}
          onSelectPacket={onSelectPacket}
          onContextMenu={handlePacketContextMenu}
        />
      </div>
      <div className="w-[480px] min-w-[380px] shrink-0">
        <PacketDetail packet={selectedPacket} />
      </div>

      {showSaveDialog && selectedPacket && (
        <SaveDialog
          packet={selectedPacket}
          onSave={handleSavePacket}
          onClose={() => setShowSaveDialog(false)}
        />
      )}

      {showExportDialog && (
        <ExportDialog packets={packets} onClose={() => setShowExportDialog(false)} />
      )}
    </div>
  );
}

function SaveDialog({
  packet,
  onSave,
  onClose,
}: {
  packet: CapturedPacket;
  onSave: (name: string, description: string, tags: string[]) => void;
  onClose: () => void;
}) {
  const proto = packet.tcp ? "TCP" : packet.udp ? "UDP" : "IP";
  const dstIp = packet.ipv4?.dst_ip || packet.ipv6?.dst_ip || "?";
  const dstPort = packet.tcp?.dst_port ?? packet.udp?.dst_port ?? "";
  const defaultName = `${proto} → ${dstIp}${dstPort ? `:${dstPort}` : ""}`;

  const [name, setName] = useState(defaultName);
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="panel p-4 w-[400px] space-y-3">
        <h3 className="text-sm font-semibold text-white">Save Packet</h3>
        <input
          type="text"
          placeholder="Packet name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="input w-full"
          autoFocus
        />
        <input
          type="text"
          placeholder="Description (optional)"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          className="input w-full"
        />
        <input
          type="text"
          placeholder="Tags (comma-separated)"
          value={tags}
          onChange={(e) => setTags(e.target.value)}
          className="input w-full"
        />
        <div className="flex justify-end gap-2 pt-1">
          <button onClick={onClose} className="btn btn-secondary text-xs">
            Cancel
          </button>
          <button
            onClick={() =>
              onSave(
                name,
                description,
                tags
                  .split(",")
                  .map((t) => t.trim())
                  .filter(Boolean)
              )
            }
            disabled={!name.trim()}
            className="btn btn-primary text-xs disabled:opacity-50"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

function ExportDialog({ packets, onClose }: { packets: CapturedPacket[]; onClose: () => void }) {
  const [format, setFormat] = useState<"pcap" | "json">("pcap");
  const [filename, setFilename] = useState("capture");
  const [exporting, setExporting] = useState(false);
  const { toast } = useToast();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const handleExport = async () => {
    setExporting(true);
    try {
      const ext = format === "pcap" ? ".pcap" : ".json";
      const path = `${filename}${ext}`;
      const ids = packets.map((p) => p.id);

      let result: string;
      if (format === "pcap") {
        result = await commands.exportPcap(ids, path);
      } else {
        result = await commands.exportJson(ids, path);
      }
      toast(result, "success");
      onClose();
    } catch (e) {
      toast(`Export failed: ${e}`, "error");
    }
    setExporting(false);
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="panel p-4 w-[400px] space-y-3">
        <h3 className="text-sm font-semibold text-white">Export Packets</h3>
        <p className="text-xs text-gray-500">
          {packets.length} packet{packets.length !== 1 ? "s" : ""} will be exported
        </p>
        <div className="flex gap-2">
          <button
            onClick={() => setFormat("pcap")}
            className={`btn text-xs flex-1 ${format === "pcap" ? "btn-primary" : "btn-secondary"}`}
          >
            PCAP
          </button>
          <button
            onClick={() => setFormat("json")}
            className={`btn text-xs flex-1 ${format === "json" ? "btn-primary" : "btn-secondary"}`}
          >
            JSON
          </button>
        </div>
        <input
          type="text"
          placeholder="Filename (without extension)"
          value={filename}
          onChange={(e) => setFilename(e.target.value)}
          className="input w-full"
          autoFocus
        />
        <p className="text-[10px] text-gray-600 font-mono">
          Saves to: {filename}
          {format === "pcap" ? ".pcap" : ".json"}
        </p>
        <div className="flex justify-end gap-2 pt-1">
          <button onClick={onClose} className="btn btn-secondary text-xs">
            Cancel
          </button>
          <button
            onClick={handleExport}
            disabled={exporting || !filename.trim()}
            className="btn btn-primary text-xs disabled:opacity-50"
          >
            {exporting ? "Exporting..." : "Export"}
          </button>
        </div>
      </div>
    </div>
  );
}

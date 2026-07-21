import { useState, useEffect, useMemo, useRef } from "react";
import { commands } from "../commands";
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
  const debounceRef = useRef<number | null>(null);

  useEffect(() => {
    loadInterfaces();
  }, []);

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

  const loadInterfaces = async () => {
    try {
      const ifaces = await commands.listInterfaces();
      setInterfaces(ifaces);
      if (ifaces.length > 0 && !selectedInterface) {
        setSelectedInterface(ifaces[0]);
      }
    } catch (e) {
      setStatusMessage(`Failed to list interfaces: ${e}`);
    }
  };

  const handleStartCapture = async () => {
    if (!selectedInterface) {
      setStatusMessage("No interface selected");
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
      setStatusMessage(`Capturing on ${config.interface_name}`);
    } catch (e) {
      setStatusMessage(`Failed to start capture: ${e}`);
    }
  };

  const handleStopCapture = async () => {
    try {
      await commands.stopCapture();
      setIsCapturing(false);
      setStatusMessage(`Capture stopped. ${packets.length} packets captured.`);
    } catch (e) {
      setStatusMessage(`Failed to stop capture: ${e}`);
    }
  };

  const handleSavePacket = async (name: string, description: string, tags: string[]) => {
    if (!selectedPacket) return;
    try {
      await commands.savePacket(selectedPacket.id, name, description, tags);
      setStatusMessage(`Packet saved as "${name}"`);
      setShowSaveDialog(false);
      onSaved();
    } catch (e) {
      setStatusMessage(`Failed to save packet: ${e}`);
    }
  };

  const handleClear = () => {
    if (confirmClear) {
      onClearPackets();
      setConfirmClear(false);
    } else {
      setConfirmClear(true);
      setTimeout(() => setConfirmClear(false), 3000);
    }
  };

  const filteredPackets = useMemo(() => packets.filter((p) => {
    if (!debouncedFilter) return true;
    const lower = debouncedFilter.toLowerCase();
    return (
      p.payload_hex.toLowerCase().includes(lower) ||
      p.payload_ascii.toLowerCase().includes(lower) ||
      (p.ipv4?.src_ip.includes(lower)) ||
      (p.ipv4?.dst_ip.includes(lower)) ||
      (p.ipv6?.src_ip?.includes(lower)) ||
      (p.ipv6?.dst_ip?.includes(lower)) ||
      (p.tcp?.src_port.toString().includes(lower)) ||
      (p.tcp?.dst_port.toString().includes(lower)) ||
      (p.udp?.src_port.toString().includes(lower)) ||
      (p.udp?.dst_port.toString().includes(lower))
    );
  }), [packets, debouncedFilter]);

  return (
    <div className="flex h-full">
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
          <input
            type="text"
            placeholder="Filter packets..."
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
            className="input text-xs flex-1 max-w-[200px]"
          />
          <button onClick={handleClear} className={`btn text-xs ${confirmClear ? "btn-danger" : "btn-secondary"}`}>
            {confirmClear ? "Confirm?" : "Clear"}
          </button>
          {selectedPacket && (
            <button
              onClick={() => setShowSaveDialog(true)}
              className="btn btn-primary text-xs"
            >
              Save
            </button>
          )}
          {packets.length > 0 && (
            <button
              onClick={() => setShowExportDialog(true)}
              className="btn btn-secondary text-xs"
            >
              Export
            </button>
          )}
        </div>
        <PacketList
          packets={filteredPackets}
          selectedPacket={selectedPacket}
          onSelectPacket={onSelectPacket}
        />
      </div>
      <div className="w-[480px] min-w-[380px] shrink-0">
        <PacketDetail
          packet={selectedPacket}
        />
      </div>

      {showSaveDialog && (
        <SaveDialog
          onSave={handleSavePacket}
          onClose={() => setShowSaveDialog(false)}
        />
      )}

      {showExportDialog && (
        <ExportDialog
          packets={packets}
          onClose={() => setShowExportDialog(false)}
          setStatusMessage={setStatusMessage}
        />
      )}
    </div>
  );
}

function SaveDialog({
  onSave,
  onClose,
}: {
  onSave: (name: string, description: string, tags: string[]) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState("");
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
            onClick={() => onSave(name, description, tags.split(",").map((t) => t.trim()).filter(Boolean))}
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

function ExportDialog({
  packets,
  onClose,
  setStatusMessage,
}: {
  packets: CapturedPacket[];
  onClose: () => void;
  setStatusMessage: (msg: string) => void;
}) {
  const [format, setFormat] = useState<"pcap" | "json">("pcap");
  const [filename, setFilename] = useState("capture");
  const [exporting, setExporting] = useState(false);

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
      setStatusMessage(result);
      onClose();
    } catch (e) {
      setStatusMessage(`Export failed: ${e}`);
    }
    setExporting(false);
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="panel p-4 w-[400px] space-y-3">
        <h3 className="text-sm font-semibold text-white">Export Packets</h3>
        <p className="text-xs text-gray-500">{packets.length} packet{packets.length !== 1 ? "s" : ""} will be exported</p>
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
          Saves to: {filename}{format === "pcap" ? ".pcap" : ".json"}
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

import { useState, useEffect, useRef, useCallback } from "react";
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
  setPackets: React.Dispatch<React.SetStateAction<CapturedPacket[]>>;
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
  setPackets,
}: CaptureViewProps) {
  const [interfaces, setInterfaces] = useState<string[]>([]);
  const [selectedInterface, setSelectedInterface] = useState("");
  const [bpfFilter, setBpfFilter] = useState("");
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [filterText, setFilterText] = useState("");

  useEffect(() => {
    loadInterfaces();
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

  const filteredPackets = packets.filter((p) => {
    if (!filterText) return true;
    const lower = filterText.toLowerCase();
    return (
      p.payload_hex.toLowerCase().includes(lower) ||
      p.payload_ascii.toLowerCase().includes(lower) ||
      (p.ipv4?.src_ip.includes(lower)) ||
      (p.ipv4?.dst_ip.includes(lower)) ||
      (p.tcp?.src_port.toString().includes(lower)) ||
      (p.tcp?.dst_port.toString().includes(lower)) ||
      (p.udp?.src_port.toString().includes(lower)) ||
      (p.udp?.dst_port.toString().includes(lower))
    );
  });

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
          <button onClick={onClearPackets} className="btn btn-secondary text-xs">
            Clear
          </button>
          {selectedPacket && (
            <button
              onClick={() => setShowSaveDialog(true)}
              className="btn btn-primary text-xs"
            >
              Save
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
          onSaved={onSaved}
          setStatusMessage={setStatusMessage}
        />
      </div>

      {showSaveDialog && (
        <SaveDialog
          onSave={handleSavePacket}
          onClose={() => setShowSaveDialog(false)}
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

import { useState } from "react";
import { commands } from "../commands";
import type { SavedPacket } from "../types";

interface LibraryViewProps {
  savedPackets: SavedPacket[];
  onRefresh: () => void;
  onSelectPacket: (p: SavedPacket) => void;
  setStatusMessage: (msg: string) => void;
}

export function LibraryView({
  savedPackets,
  onRefresh,
  onSelectPacket,
  setStatusMessage,
}: LibraryViewProps) {
  const [searchText, setSearchText] = useState("");
  const [exporting, setExporting] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<{ id: string; name: string } | null>(null);

  const filtered = savedPackets.filter((p) => {
    if (!searchText) return true;
    const lower = searchText.toLowerCase();
    return (
      p.name.toLowerCase().includes(lower) ||
      p.src_endpoint.toLowerCase().includes(lower) ||
      p.dst_endpoint.toLowerCase().includes(lower) ||
      p.protocol.toLowerCase().includes(lower) ||
      p.tags.some((t) => t.toLowerCase().includes(lower))
    );
  });

  const handleDelete = async (id: string, _name: string) => {
    try {
      await commands.deleteSavedPacket(id);
      onRefresh();
      setStatusMessage("Packet deleted");
    } catch (e) {
      setStatusMessage(`Delete failed: ${e}`);
    }
  };

  const handleDeleteClick = (id: string, name: string) => {
    if (pendingDelete?.id === id) {
      handleDelete(id, name);
      setPendingDelete(null);
    } else {
      setPendingDelete({ id, name });
      setTimeout(() => setPendingDelete(null), 3000);
    }
  };

  const handleExportAll = async () => {
    if (filtered.length === 0) return;
    setExporting(true);
    try {
      const ids = filtered.map((p) => p.id);
      const result = await commands.exportJson(ids, "packets_export.json");
      setStatusMessage(result);
    } catch (e) {
      setStatusMessage(`Export failed: ${e}`);
    }
    setExporting(false);
  };

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center gap-2 p-3 bg-[#0d1117] border-b border-[#1e293b]">
        <input
          type="text"
          placeholder="Search saved packets..."
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          className="input text-xs flex-1 max-w-[300px]"
        />
        <div className="flex-1" />
        <button
          onClick={handleExportAll}
          disabled={exporting || filtered.length === 0}
          className="btn btn-secondary text-xs disabled:opacity-50"
        >
          Export JSON
        </button>
      </div>

      <div className="flex-1 overflow-auto p-3">
        {filtered.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-600 text-sm">
            {savedPackets.length === 0
              ? "No saved packets yet. Capture packets and save them from the Capture view."
              : "No packets match your search."}
          </div>
        ) : (
          <div className="space-y-2">
            {filtered.map((p) => (
              <div
                key={p.id}
                className="panel p-3 hover:bg-[#161b22] cursor-pointer transition-colors"
                onClick={() => onSelectPacket(p)}
              >
                <div className="flex items-center gap-3">
                  <span
                    className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
                      p.protocol === "TCP"
                        ? "bg-blue-900/40 text-blue-400"
                        : p.protocol === "UDP"
                        ? "bg-green-900/40 text-green-400"
                        : "bg-gray-800 text-gray-400"
                    }`}
                  >
                    {p.protocol}
                  </span>
                  <span className="text-sm text-white font-medium">{p.name}</span>
                  <span className="text-xs text-gray-500">
                    {p.src_endpoint} → {p.dst_endpoint}
                  </span>
                  <div className="flex-1" />
                  {p.tags.length > 0 && (
                    <div className="flex gap-1">
                      {p.tags.map((tag) => (
                        <span
                          key={tag}
                          className="text-[10px] px-1.5 py-0.5 rounded bg-gray-800 text-gray-500"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}
                  <span className="text-[10px] text-gray-600">
                    {p.raw_bytes.length} bytes
                  </span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteClick(p.id, p.name);
                    }}
                    className={`btn text-[10px] px-2 py-0.5 ${
                      pendingDelete?.id === p.id ? "btn-danger opacity-100" : "btn-danger opacity-50 hover:opacity-100"
                    }`}
                  >
                    {pendingDelete?.id === p.id ? "Sure?" : "×"}
                  </button>
                </div>
                {p.description && (
                  <p className="text-xs text-gray-500 mt-1">{p.description}</p>
                )}
                <div className="text-[10px] text-gray-600 mt-1 font-mono">
                  {p.payload_hex.substring(0, 120)}
                  {p.payload_hex.length > 120 ? "…" : ""}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

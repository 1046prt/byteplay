import { useState } from "react";
import { commands } from "../commands";
import { DEFAULT_PORT, DEFAULT_TIMEOUT_MS } from "../constants";
import { useToast } from "./Toast";
import type { SavedPacket, SequenceStep, SavedSequence, ReplayResult } from "../types";

interface SequenceViewProps {
  sequences: SavedSequence[];
  savedPackets: SavedPacket[];
  onRefresh: () => void;
  setStatusMessage: (msg: string) => void;
}

export function SequenceView({ sequences, savedPackets, onRefresh }: SequenceViewProps) {
  const [steps, setSteps] = useState<SequenceStep[]>([]);
  const [sequenceName, setSequenceName] = useState("");
  const [sequenceDesc, setSequenceDesc] = useState("");
  const [results, setResults] = useState<ReplayResult[] | null>(null);
  const [executing, setExecuting] = useState(false);
  const [allowExternal, setAllowExternal] = useState(false);
  const { toast } = useToast();

  const addStepFromPacket = (packet: SavedPacket) => {
    const newStep: SequenceStep = {
      name: packet.name,
      target_host: "127.0.0.1",
      target_port: extractPort(packet.dst_endpoint),
      protocol: packet.protocol,
      data: packet.raw_bytes,
      delay_ms: 0,
      timeout_ms: DEFAULT_TIMEOUT_MS,
    };
    setSteps([...steps, newStep]);
    toast(`Added step: ${packet.name}`, "success");
  };

  const loadSequence = (seq: SavedSequence) => {
    setSteps(seq.steps);
    setSequenceName(seq.name);
    setSequenceDesc(seq.description);
    toast(`Loaded: ${seq.name} (${seq.steps.length} steps)`, "info");
  };

  const extractPort = (endpoint: string): number => {
    const parts = endpoint.split(":");
    const port = parseInt(parts[parts.length - 1], 10);
    return isNaN(port) ? DEFAULT_PORT : port;
  };

  const removeStep = (index: number) => {
    setSteps(steps.filter((_, i) => i !== index));
  };

  const updateStep = (index: number, field: keyof SequenceStep, value: string | number) => {
    const updated = [...steps];
    updated[index] = { ...updated[index], [field]: value };
    setSteps(updated);
  };

  const moveStep = (index: number, direction: -1 | 1) => {
    const newIndex = index + direction;
    if (newIndex < 0 || newIndex >= steps.length) return;
    const updated = [...steps];
    [updated[index], updated[newIndex]] = [updated[newIndex], updated[index]];
    setSteps(updated);
  };

  const executeSequence = async () => {
    if (steps.length === 0) return;
    setExecuting(true);
    setResults(null);
    try {
      const res = await commands.executeReplaySequence(steps, allowExternal);
      setResults(res);
      const successCount = res.filter((r) => r.success).length;
      toast(
        `Sequence: ${successCount}/${res.length} succeeded`,
        successCount === res.length ? "success" : "error"
      );
      onRefresh();
    } catch (e) {
      toast(`Sequence failed: ${e}`, "error");
    }
    setExecuting(false);
  };

  const saveSequence = async () => {
    if (!sequenceName.trim() || steps.length === 0) return;
    try {
      await commands.saveSequence(sequenceName, sequenceDesc, steps, []);
      toast(`Sequence "${sequenceName}" saved`, "success");
      setSequenceName("");
      setSequenceDesc("");
      setSteps([]);
      onRefresh();
    } catch (e) {
      toast(`Failed to save: ${e}`, "error");
    }
  };

  const deleteSequence = async (id: string, name: string) => {
    try {
      await commands.deleteSequence(id);
      toast(`Sequence "${name}" deleted`, "info");
      onRefresh();
    } catch (e) {
      toast(`Failed to delete: ${e}`, "error");
    }
  };

  return (
    <div className="h-full flex">
      <div className="flex-1 flex flex-col min-w-0">
        <div className="p-3 bg-[#0d1117] border-b border-[#1e293b] space-y-3">
          <div className="flex items-center gap-3">
            <h3 className="text-sm font-semibold text-white">Sequence Builder</h3>
            <span className="text-[10px] text-gray-600">{steps.length} steps</span>
            <div className="flex-1" />
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={allowExternal}
                onChange={(e) => setAllowExternal(e.target.checked)}
              />
              Allow external
            </label>
            <button
              onClick={() => {
                setSteps([]);
                setSequenceName("");
                setSequenceDesc("");
                setResults(null);
              }}
              className="btn btn-secondary text-xs"
            >
              Clear
            </button>
            <button
              onClick={executeSequence}
              disabled={executing || steps.length === 0}
              className="btn btn-success text-xs disabled:opacity-50"
            >
              {executing ? "Running..." : "▶ Run"}
            </button>
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Sequence name"
              value={sequenceName}
              onChange={(e) => setSequenceName(e.target.value)}
              className="input text-xs flex-1"
            />
            <input
              type="text"
              placeholder="Description"
              value={sequenceDesc}
              onChange={(e) => setSequenceDesc(e.target.value)}
              className="input text-xs flex-1"
            />
            <button
              onClick={saveSequence}
              disabled={!sequenceName.trim() || steps.length === 0}
              className="btn btn-primary text-xs disabled:opacity-50"
            >
              Save
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-auto p-3">
          {steps.length === 0 ? (
            <div className="text-center text-gray-600 text-sm mt-8">
              <p>No steps yet.</p>
              <p className="text-xs mt-2 text-gray-700">
                Add packets from the panel on the right, or load a saved sequence.
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {steps.map((step, i) => (
                <div key={i} className="panel p-3">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-[10px] text-gray-600 font-mono w-[20px]">{i + 1}.</span>
                    <span className="text-xs text-white font-medium">{step.name}</span>
                    <div className="flex-1" />
                    <button
                      onClick={() => moveStep(i, -1)}
                      disabled={i === 0}
                      className="text-gray-600 hover:text-gray-300 text-xs px-1 disabled:opacity-30"
                    >
                      ↑
                    </button>
                    <button
                      onClick={() => moveStep(i, 1)}
                      disabled={i === steps.length - 1}
                      className="text-gray-600 hover:text-gray-300 text-xs px-1 disabled:opacity-30"
                    >
                      ↓
                    </button>
                    <button
                      onClick={() => removeStep(i)}
                      className="text-red-500 hover:text-red-400 text-xs px-1"
                    >
                      ×
                    </button>
                  </div>
                  <div className="grid grid-cols-4 gap-2 text-[10px]">
                    <div>
                      <label className="text-gray-600 block mb-0.5">Host</label>
                      <input
                        type="text"
                        value={step.target_host}
                        onChange={(e) => updateStep(i, "target_host", e.target.value)}
                        className="input input-mono text-[10px] w-full"
                      />
                    </div>
                    <div>
                      <label className="text-gray-600 block mb-0.5">Port</label>
                      <input
                        type="number"
                        min="1"
                        max="65535"
                        value={String(step.target_port)}
                        onChange={(e) =>
                          updateStep(i, "target_port", parseInt(e.target.value) || 0)
                        }
                        className="input input-mono text-[10px] w-full"
                      />
                    </div>
                    <div>
                      <label className="text-gray-600 block mb-0.5">Protocol</label>
                      <select
                        value={step.protocol}
                        onChange={(e) => updateStep(i, "protocol", e.target.value)}
                        className="input text-[10px] w-full"
                      >
                        <option value="TCP">TCP</option>
                        <option value="UDP">UDP</option>
                      </select>
                    </div>
                    <div>
                      <label className="text-gray-600 block mb-0.5">Delay (ms)</label>
                      <input
                        type="number"
                        min="0"
                        value={String(step.delay_ms)}
                        onChange={(e) => updateStep(i, "delay_ms", parseInt(e.target.value) || 0)}
                        className="input input-mono text-[10px] w-full"
                      />
                    </div>
                  </div>
                  <div className="text-[10px] text-gray-600 mt-1 font-mono truncate">
                    {step.data.length} bytes
                  </div>
                </div>
              ))}
            </div>
          )}

          {results && (
            <div className="mt-4 space-y-2">
              <h4 className="text-xs font-semibold text-gray-400">Results</h4>
              {results.map((r, i) => (
                <div
                  key={i}
                  className={`panel p-2 text-xs ${
                    r.success ? "border-green-900/30" : "border-red-900/30"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className="text-gray-600">Step {i + 1}</span>
                    <span className={r.success ? "text-green-400" : "text-red-400"}>
                      {r.success ? "OK" : "FAIL"}
                    </span>
                    <span className="text-gray-600">
                      {r.bytes_sent} bytes, {r.duration_ms}ms
                    </span>
                    {r.error && <span className="text-red-400 truncate">{r.error}</span>}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="w-[280px] border-l border-[#1e293b] flex flex-col">
        <div className="p-2 bg-[#0d1117] border-b border-[#1e293b]">
          <h3 className="text-xs font-semibold text-gray-400">Saved Packets (click to add)</h3>
        </div>
        <div className="flex-1 overflow-auto">
          {savedPackets.length === 0 ? (
            <div className="p-3 text-xs text-gray-600 text-center">
              No saved packets. Save some from the Capture view first.
            </div>
          ) : (
            <div className="space-y-1 p-2">
              {savedPackets.map((p) => (
                <button
                  key={p.id}
                  onClick={() => addStepFromPacket(p)}
                  className="w-full text-left p-2 rounded bg-[#111827] hover:bg-[#1a2236] text-[10px] space-y-0.5 transition-colors"
                >
                  <div className="flex items-center gap-1">
                    <span className="text-gray-600">+</span>
                    <span className="text-white">{p.name}</span>
                    <span className="text-gray-600">{p.protocol}</span>
                  </div>
                  <div className="text-gray-600 font-mono truncate">
                    {p.src_endpoint} → {p.dst_endpoint}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {sequences.length > 0 && (
          <>
            <div className="p-2 bg-[#0d1117] border-t border-[#1e293b]">
              <h3 className="text-xs font-semibold text-gray-400">Saved Sequences</h3>
            </div>
            <div className="max-h-[200px] overflow-auto">
              {sequences.map((s) => (
                <div
                  key={s.id}
                  onClick={() => loadSequence(s)}
                  className="p-2 border-t border-[#111827] text-[10px] hover:bg-[#1a2236] cursor-pointer transition-colors"
                >
                  <div className="flex items-center gap-2">
                    <span className="text-white">{s.name}</span>
                    <span className="text-gray-600">{s.steps.length} steps</span>
                    <div className="flex-1" />
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteSequence(s.id, s.name);
                      }}
                      className="text-red-500 hover:text-red-400"
                    >
                      ×
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

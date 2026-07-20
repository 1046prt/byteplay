import { useState } from "react";
import { commands } from "../commands";
import {
  DEFAULT_PORT,
  DEFAULT_FUZZ_TIMEOUT_MS,
  DEFAULT_FUZZ_ITERATIONS,
  DEFAULT_FUZZ_MUTATION_RATE,
} from "../constants";
import type { CapturedPacket, FuzzConfig, FuzzResult } from "../types";
import { HexViewer } from "./HexViewer";

interface FuzzerViewProps {
  selectedPacket: CapturedPacket | null;
  setStatusMessage: (msg: string) => void;
}

export function FuzzerView({ selectedPacket, setStatusMessage }: FuzzerViewProps) {
  const [targetHost, setTargetHost] = useState("127.0.0.1");
  const [targetPort, setTargetPort] = useState(String(DEFAULT_PORT));
  const [protocol, setProtocol] = useState("TCP");
  const [iterations, setIterations] = useState(String(DEFAULT_FUZZ_ITERATIONS));
  const [mutationRate, setMutationRate] = useState(String(DEFAULT_FUZZ_MUTATION_RATE));
  const [timeout, setTimeout_] = useState(String(DEFAULT_FUZZ_TIMEOUT_MS));
  const [allowExternal, setAllowExternal] = useState(false);
  const [customPayload, setCustomPayload] = useState("");
  const [useCustom, setUseCustom] = useState(false);

  const [results, setResults] = useState<FuzzResult[] | null>(null);
  const [running, setRunning] = useState(false);
  const [selectedResult, setSelectedResult] = useState<FuzzResult | null>(null);

  const handleFuzz = async () => {
    setRunning(true);
    setResults(null);
    setSelectedResult(null);
    try {
      const basePayload = useCustom
        ? Array.from(new TextEncoder().encode(customPayload))
        : selectedPacket?.raw_bytes || [];

      if (basePayload.length === 0) {
        setStatusMessage("No payload to fuzz");
        setRunning(false);
        return;
      }

      const config: FuzzConfig = {
        target_host: targetHost,
        target_port: parseInt(targetPort, 10),
        protocol,
        base_payload: basePayload,
        iterations: parseInt(iterations, 10),
        mutation_rate: parseFloat(mutationRate),
        timeout_ms: parseInt(timeout, 10),
        allow_external: allowExternal,
      };

      const res = await commands.runFuzzer(config);
      setResults(res);

      const crashes = res.filter((r) => !r.replay_result.success).length;
      const responses = res.filter((r) => r.replay_result.success && r.replay_result.response).length;
      setStatusMessage(
        `Fuzz complete: ${crashes} errors, ${responses} responses out of ${res.length} iterations`
      );
    } catch (e) {
      setStatusMessage(`Fuzz error: ${e}`);
    }
    setRunning(false);
  };

  return (
    <div className="h-full flex">
      <div className="w-[340px] flex flex-col border-r border-[#1e293b]">
        <div className="p-3 bg-[#0d1117] border-b border-[#1e293b] space-y-3">
          <h3 className="text-sm font-semibold text-white">Fuzzing Configuration</h3>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Target Host</label>
              <input
                type="text"
                value={targetHost}
                onChange={(e) => setTargetHost(e.target.value)}
                className="input input-mono text-xs w-full"
              />
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Target Port</label>
              <input
                type="text"
                value={targetPort}
                onChange={(e) => setTargetPort(e.target.value)}
                className="input input-mono text-xs w-full"
              />
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Protocol</label>
              <select
                value={protocol}
                onChange={(e) => setProtocol(e.target.value)}
                className="input text-xs w-full"
              >
                <option value="TCP">TCP</option>
                <option value="UDP">UDP</option>
              </select>
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Timeout (ms)</label>
              <input
                type="text"
                value={timeout}
                onChange={(e) => setTimeout_(e.target.value)}
                className="input input-mono text-xs w-full"
              />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Iterations</label>
              <input
                type="number"
                value={iterations}
                onChange={(e) => setIterations(e.target.value)}
                className="input input-mono text-xs w-full"
                min="1"
                max="1000"
              />
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Mutation Rate</label>
              <input
                type="number"
                value={mutationRate}
                onChange={(e) => setMutationRate(e.target.value)}
                className="input input-mono text-xs w-full"
                min="0.01"
                max="1.0"
                step="0.05"
              />
            </div>
          </div>
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={useCustom}
                onChange={(e) => setUseCustom(e.target.checked)}
              />
              Custom payload
            </label>
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={allowExternal}
                onChange={(e) => setAllowExternal(e.target.checked)}
              />
              Allow external
            </label>
          </div>
          {useCustom && (
            <textarea
              value={customPayload}
              onChange={(e) => setCustomPayload(e.target.value)}
              placeholder="Base payload for fuzzing"
              className="w-full h-[80px] bg-[#0a0e17] border border-[#1e293b] rounded-md p-2 font-mono text-xs text-gray-300 resize-none outline-none"
            />
          )}
          <button
            onClick={handleFuzz}
            disabled={running}
            className="btn btn-primary text-xs w-full disabled:opacity-50"
          >
            {running ? (
              <span className="flex items-center justify-center gap-2">
                <span className="animate-pulse">●</span>
                Fuzzing {iterations} iterations...
              </span>
            ) : (
              `⚡ Run ${iterations} Fuzz Iterations`
            )}
          </button>
        </div>

        <div className="flex-1 overflow-auto p-3">
          {selectedResult && (
            <div className="space-y-3">
              <div className="panel p-3">
                <h4 className="text-xs font-semibold text-gray-400 mb-2">
                  Iteration #{selectedResult.iteration}
                </h4>
                <div className="text-xs space-y-1">
                  <div>
                    <span className="text-gray-600">Status: </span>
                    <span
                      className={
                        selectedResult.replay_result.success
                          ? "text-green-400"
                          : "text-red-400"
                      }
                    >
                      {selectedResult.replay_result.success ? "OK" : "Error"}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-600">Duration: </span>
                    <span className="text-gray-300">
                      {selectedResult.replay_result.duration_ms}ms
                    </span>
                  </div>
                  {selectedResult.replay_result.error && (
                    <div className="text-red-400 font-mono text-[10px]">
                      {selectedResult.replay_result.error}
                    </div>
                  )}
                </div>
              </div>
              <div className="panel p-3">
                <h4 className="text-xs font-semibold text-gray-400 mb-1">
                  Mutations ({selectedResult.mutations_applied.length})
                </h4>
                <div className="space-y-0.5 max-h-[120px] overflow-auto">
                  {selectedResult.mutations_applied.map((m, i) => (
                    <div key={i} className="text-[10px] font-mono text-gray-500">
                      <span className="text-gray-600">
                        0x{m.offset.toString(16).padStart(4, "0")}
                      </span>{" "}
                      <span className="text-gray-400">
                        0x{m.original.toString(16).padStart(2, "0")}
                      </span>
                      <span className="text-blue-400"> → </span>
                      <span className="text-yellow-400">
                        0x{m.mutated.toString(16).padStart(2, "0")}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
              {selectedResult.replay_result.response && (
                <div className="panel p-3">
                  <h4 className="text-xs font-semibold text-gray-400 mb-2">Response</h4>
                  <HexViewer bytes={selectedResult.replay_result.response} />
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="flex-1 flex flex-col min-w-0">
        <div className="p-2 bg-[#0d1117] border-b border-[#1e293b] flex items-center gap-3">
          <h3 className="text-xs font-semibold text-gray-400">Results</h3>
          {results && (
            <>
              <span className="text-[10px] text-gray-600">
                {results.filter((r) => r.replay_result.success).length} OK
              </span>
              <span className="text-[10px] text-red-400">
                {results.filter((r) => !r.replay_result.success).length} Errors
              </span>
              <span className="text-[10px] text-gray-600">
                {results.filter((r) => r.replay_result.response).length} Responses
              </span>
            </>
          )}
        </div>
        <div className="flex-1 overflow-auto p-3">
          {!results ? (
            <div className="flex items-center justify-center h-full text-gray-600 text-sm">
              Configure and run the fuzzer to see results
            </div>
          ) : (
            <div className="space-y-1">
              {results.map((r) => (
                <div
                  key={r.iteration}
                  onClick={() => setSelectedResult(r)}
                  className={`flex items-center gap-3 p-2 rounded text-xs font-mono cursor-pointer transition-colors ${
                    selectedResult?.iteration === r.iteration
                      ? "bg-blue-600/20 border border-blue-600/30"
                      : "bg-[#111827] hover:bg-[#1a2236] border border-transparent"
                  } ${
                    !r.replay_result.success
                      ? "border-l-2 border-l-red-500"
                      : r.replay_result.response
                      ? "border-l-2 border-l-green-500"
                      : ""
                  }`}
                >
                  <span className="w-[40px] text-gray-600">#{r.iteration}</span>
                  <span
                    className={
                      r.replay_result.success ? "text-green-400" : "text-red-400"
                    }
                  >
                    {r.replay_result.success ? "OK" : "ERR"}
                  </span>
                  <span className="text-gray-500">
                    {r.replay_result.duration_ms}ms
                  </span>
                  <span className="text-gray-600">
                    {r.mutations_applied.length} muts
                  </span>
                  <span className="text-gray-700 truncate flex-1">
                    {r.replay_result.error || ""}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

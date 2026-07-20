import { useState } from "react";
import { commands } from "../commands";
import { DEFAULT_PORT, DEFAULT_TIMEOUT_MS } from "../constants";
import type { CapturedPacket, ReplayConfig, ReplayResult, ReplayRecord } from "../types";
import { HexViewer } from "./HexViewer";

interface ReplayViewProps {
  selectedPacket: CapturedPacket | null;
  replayHistory: ReplayRecord[];
  onRefreshHistory: () => void;
  setStatusMessage: (msg: string) => void;
}

export function ReplayView({
  selectedPacket,
  replayHistory,
  onRefreshHistory,
  setStatusMessage,
}: ReplayViewProps) {
  const [targetHost, setTargetHost] = useState("127.0.0.1");
  const [targetPort, setTargetPort] = useState(String(DEFAULT_PORT));
  const [protocol, setProtocol] = useState("TCP");
  const [timeout, setTimeout_] = useState(String(DEFAULT_TIMEOUT_MS));
  const [allowExternal, setAllowExternal] = useState(false);
  const [customPayload, setCustomPayload] = useState("");
  const [useCustom, setUseCustom] = useState(false);
  const [result, setResult] = useState<ReplayResult | null>(null);
  const [sending, setSending] = useState(false);

  const handleReplay = async () => {
    setSending(true);
    setResult(null);
    try {
      const data = useCustom
        ? Array.from(new TextEncoder().encode(customPayload))
        : selectedPacket?.raw_bytes || [];

      if (data.length === 0) {
        setStatusMessage("No data to send");
        setSending(false);
        return;
      }

      const config: ReplayConfig = {
        target_host: targetHost,
        target_port: parseInt(targetPort, 10),
        protocol: protocol,
        timeout_ms: parseInt(timeout, 10),
        allow_external: allowExternal,
      };

      const res = await commands.replayPacket(data, config);
      setResult(res);

      if (res.success) {
        setStatusMessage(`Replay OK: ${res.bytes_sent} bytes sent in ${res.duration_ms}ms`);
      } else {
        setStatusMessage(`Replay failed: ${res.error}`);
      }

      onRefreshHistory();
    } catch (e) {
      setStatusMessage(`Replay error: ${e}`);
    }
    setSending(false);
  };

  return (
    <div className="h-full flex">
      <div className="flex-1 flex flex-col min-w-0">
        <div className="p-3 bg-[#0d1117] border-b border-[#1e293b] space-y-3">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-white">Replay Target</h3>
            {allowExternal && (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-red-900/40 text-red-400">
                External allowed
              </span>
            )}
          </div>
          <div className="grid grid-cols-4 gap-2">
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Host</label>
              <input
                type="text"
                value={targetHost}
                onChange={(e) => setTargetHost(e.target.value)}
                className="input input-mono text-xs w-full"
              />
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">Port</label>
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
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={useCustom}
                onChange={(e) => setUseCustom(e.target.checked)}
                className="rounded"
              />
              Use custom payload
            </label>
            <label className="flex items-center gap-2 text-xs text-gray-400">
              <input
                type="checkbox"
                checked={allowExternal}
                onChange={(e) => setAllowExternal(e.target.checked)}
                className="rounded"
              />
              Allow external targets
            </label>
            <div className="flex-1" />
            <button
              onClick={handleReplay}
              disabled={sending}
              className="btn btn-primary text-xs disabled:opacity-50"
            >
              {sending ? "Sending..." : "▶ Send"}
            </button>
          </div>
          {useCustom && (
            <textarea
              value={customPayload}
              onChange={(e) => setCustomPayload(e.target.value)}
              placeholder="Enter custom payload (raw text/bytes)"
              className="w-full h-[100px] bg-[#0a0e17] border border-[#1e293b] rounded-md p-2 font-mono text-xs text-gray-300 resize-none outline-none"
            />
          )}
        </div>

        <div className="flex-1 overflow-auto p-3">
          {result && (
            <div className="space-y-3">
              <div className="panel p-3">
                <h4 className="text-xs font-semibold text-gray-400 mb-2">Result</h4>
                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div>
                    <span className="text-gray-600">Status: </span>
                    <span className={result.success ? "text-green-400" : "text-red-400"}>
                      {result.success ? "Success" : "Failed"}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-600">Sent: </span>
                    <span className="text-gray-300">{result.bytes_sent} bytes</span>
                  </div>
                  <div>
                    <span className="text-gray-600">Time: </span>
                    <span className="text-gray-300">{result.duration_ms}ms</span>
                  </div>
                </div>
                {result.error && (
                  <div className="text-xs text-red-400 mt-2 font-mono">
                    {result.error}
                  </div>
                )}
              </div>

              {result.response && (
                <div className="panel p-3">
                  <h4 className="text-xs font-semibold text-gray-400 mb-2">
                    Response ({result.response.length} bytes)
                  </h4>
                  <HexViewer bytes={result.response} />
                  {result.response_hex && (
                    <div className="mt-2">
                      <h4 className="text-[10px] text-gray-500 mb-1">ASCII</h4>
                      <div className="font-mono text-xs text-gray-400 whitespace-pre-wrap break-all">
                        {result.response.map((b) =>
                          b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : "."
                        ).join("")}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="w-[300px] border-l border-[#1e293b] flex flex-col">
        <div className="p-2 bg-[#0d1117] border-b border-[#1e293b]">
          <h3 className="text-xs font-semibold text-gray-400">Replay History</h3>
        </div>
        <div className="flex-1 overflow-auto">
          {replayHistory.length === 0 ? (
            <div className="p-3 text-xs text-gray-600 text-center">
              No replay history yet
            </div>
          ) : (
            <div className="space-y-1 p-2">
              {replayHistory.map((r) => (
                <div
                  key={r.id}
                  className="p-2 rounded bg-[#111827] text-[10px] space-y-0.5"
                >
                  <div className="flex items-center gap-2">
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        r.success ? "bg-green-500" : "bg-red-500"
                      }`}
                    />
                    <span className="text-gray-300 font-mono">
                      {r.target_host}:{r.target_port}
                    </span>
                  </div>
                  <div className="text-gray-600">
                    {r.bytes_sent} bytes • {r.duration_ms}ms
                  </div>
                  {r.error && (
                    <div className="text-red-400 truncate">{r.error}</div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

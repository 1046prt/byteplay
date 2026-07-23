import { useState, useEffect, useCallback, useRef } from "react";
import { commands } from "./commands";
import { POLL_INTERVAL_MS } from "./constants";
import type {
  CapturedPacket,
  SavedPacket,
  ReplayRecord,
  SavedSequence,
  ViewMode,
  CaptureStats,
} from "./types";
import { Sidebar } from "./components/Sidebar";
import { Header } from "./components/Header";
import { CaptureView } from "./components/CaptureView";
import { LibraryView } from "./components/LibraryView";
import { ReplayView } from "./components/ReplayView";
import { SequenceView } from "./components/SequenceView";
import { FuzzerView } from "./components/FuzzerView";
import { StatsView } from "./components/StatsView";
import { ToastProvider } from "./components/Toast";
import { ContextMenuRenderer } from "./components/ContextMenu";
import { ErrorBoundary } from "./components/ErrorBoundary";

export default function App() {
  return (
    <ErrorBoundary>
      <ToastProvider>
        <AppInner />
        <ContextMenuRenderer />
      </ToastProvider>
    </ErrorBoundary>
  );
}

function AppInner() {
  const [view, setView] = useState<ViewMode>("capture");
  const [packets, setPackets] = useState<CapturedPacket[]>([]);
  const [selectedPacket, setSelectedPacket] = useState<CapturedPacket | null>(null);
  const [savedPackets, setSavedPackets] = useState<SavedPacket[]>([]);
  const [replayHistory, setReplayHistory] = useState<ReplayRecord[]>([]);
  const [sequences, setSequences] = useState<SavedSequence[]>([]);
  const [isCapturing, setIsCapturing] = useState(false);
  const [statusMessage, setStatusMessage] = useState("Ready");
  const [captureStats, setCaptureStats] = useState<CaptureStats>({
    packetsPerSecond: 0,
    bytesPerSecond: 0,
    totalBytes: 0,
    duration: 0,
  });
  const pollRef = useRef<number | null>(null);
  const packetBufferRef = useRef<CapturedPacket[]>([]);
  const statsRef = useRef({ totalPackets: 0, totalBytes: 0, startTime: 0, lastSamplePackets: 0, lastSampleBytes: 0, lastSampleTime: 0 });

  useEffect(() => {
    const base = "byteplay";
    if (isCapturing) {
      document.title = `● Capturing — ${base}`;
    } else {
      document.title = base;
    }
  }, [isCapturing]);

  const flushBuffer = useCallback(() => {
    if (packetBufferRef.current.length > 0) {
      const batch = packetBufferRef.current;
      packetBufferRef.current = [];
      setPackets((prev) => {
        const next = prev.concat(batch);
        return next.length > 10000 ? next.slice(next.length - 10000) : next;
      });
    }
  }, []);

  const pollPackets = useCallback(async () => {
    try {
      const newPackets = await commands.pollPackets();
      if (newPackets.length > 0) {
        packetBufferRef.current.push(...newPackets);

        const s = statsRef.current;
        for (const p of newPackets) {
          s.totalPackets++;
          s.totalBytes += p.frame_length;
        }
      }
      flushBuffer();

      const s = statsRef.current;
      const now = performance.now();
      if (s.lastSampleTime > 0 && now - s.lastSampleTime > 0) {
        const elapsed = (now - s.lastSampleTime) / 1000;
        const pps = (s.totalPackets - s.lastSamplePackets) / elapsed;
        const bps = (s.totalBytes - s.lastSampleBytes) / elapsed;
        const duration = s.startTime > 0 ? (now - s.startTime) / 1000 : 0;
        setCaptureStats({
          packetsPerSecond: Math.round(pps),
          bytesPerSecond: Math.round(bps),
          totalBytes: s.totalBytes,
          duration,
        });
      }
      if (s.lastSampleTime === 0 || now - s.lastSampleTime > 500) {
        s.lastSamplePackets = s.totalPackets;
        s.lastSampleBytes = s.totalBytes;
        s.lastSampleTime = now;
      }
    } catch (e) {
      console.error("Poll error:", e);
    }
  }, [flushBuffer]);

  useEffect(() => {
    if (isCapturing) {
      const s = statsRef.current;
      s.startTime = performance.now();
      s.lastSampleTime = 0;
      s.lastSamplePackets = s.totalPackets;
      s.lastSampleBytes = s.totalBytes;
      pollRef.current = window.setInterval(pollPackets, POLL_INTERVAL_MS);
    } else if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [isCapturing, pollPackets]);

  const loadSavedData = useCallback(async () => {
    try {
      const [sp, rh, seq] = await Promise.all([
        commands.getSavedPackets(),
        commands.getReplayHistory(),
        commands.getSavedSequences(),
      ]);
      setSavedPackets(sp);
      setReplayHistory(rh);
      setSequences(seq);
    } catch (e) {
      console.error("Failed to load saved data:", e);
    }
  }, []);

  useEffect(() => {
    loadSavedData();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;

      if (view === "capture") {
        if (e.key === "ArrowDown" || e.key === "j") {
          e.preventDefault();
          setPackets((prev) => {
            setSelectedPacket((curr) => {
              if (!curr && prev.length > 0) return prev[0];
              if (!curr) return null;
              const idx = prev.findIndex((p) => p.id === curr.id);
              return idx < prev.length - 1 ? prev[idx + 1] : curr;
            });
            return prev;
          });
        } else if (e.key === "ArrowUp" || e.key === "k") {
          e.preventDefault();
          setPackets((prev) => {
            setSelectedPacket((curr) => {
              if (!curr && prev.length > 0) return prev[prev.length - 1];
              if (!curr) return null;
              const idx = prev.findIndex((p) => p.id === curr.id);
              return idx > 0 ? prev[idx - 1] : curr;
            });
            return prev;
          });
        } else if (e.key === "Escape") {
          setSelectedPacket(null);
        } else if ((e.ctrlKey || e.metaKey) && e.key === "s") {
          e.preventDefault();
          if (selectedPacket) {
            window.dispatchEvent(new CustomEvent("packetforge:save-packet"));
          }
        }
      } else if (e.key === "Escape") {
        setSelectedPacket(null);
      }

      if (e.key === "1") setView("capture");
      else if (e.key === "2") setView("library");
      else if (e.key === "3") setView("replay");
      else if (e.key === "4") setView("sequences");
      else if (e.key === "5") setView("fuzzer");
      else if (e.key === "6") setView("stats");
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [view, selectedPacket]);

  const handleClearPackets = async () => {
    await commands.clearPackets();
    setPackets([]);
    setSelectedPacket(null);
    statsRef.current.totalPackets = 0;
    statsRef.current.totalBytes = 0;
    setCaptureStats({ packetsPerSecond: 0, bytesPerSecond: 0, totalBytes: 0, duration: 0 });
    setStatusMessage("Packets cleared");
  };

  const handleImportPcap = useCallback((imported: CapturedPacket[]) => {
    setPackets((prev) => [...prev, ...imported]);
    setStatusMessage(`Imported ${imported.length} packets`);
  }, [setStatusMessage]);

  return (
    <div className="flex h-screen overflow-hidden bg-[#0a0e17]">
      <Sidebar
        currentView={view}
        onNavigate={setView}
        packetCount={packets.length}
        savedCount={savedPackets.length}
        sequenceCount={sequences.length}
      />
      <div className="flex flex-col flex-1 overflow-hidden">
        <Header
          isCapturing={isCapturing}
          packetCount={packets.length}
          statusMessage={statusMessage}
          stats={captureStats}
        />
        <main className="flex-1 overflow-hidden">
          {view === "capture" && (
            <CaptureView
              packets={packets}
              selectedPacket={selectedPacket}
              onSelectPacket={setSelectedPacket}
              isCapturing={isCapturing}
              setIsCapturing={setIsCapturing}
              onClearPackets={handleClearPackets}
              onSaved={loadSavedData}
              onImportPcap={handleImportPcap}
              setStatusMessage={setStatusMessage}
            />
          )}
          {view === "library" && (
            <LibraryView
              savedPackets={savedPackets}
              onRefresh={loadSavedData}
              onSelectPacket={(p) => {
                setSelectedPacket(null);
                commands.reparsePacket(p.raw_bytes, "library").then((parsed) => {
                  setSelectedPacket(parsed);
                }).catch(() => {
                  const ascii = p.payload.map((b) => (b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : ".")).join("");
                  setSelectedPacket({
                    id: p.id,
                    timestamp: p.timestamp,
                    interface: "library",
                    frame_length: p.raw_bytes.length,
                    ethernet: null,
                    ipv4: null,
                    ipv6: null,
                    tcp: p.protocol === "TCP" ? { src_port: parseInt(p.src_endpoint.split(":").pop() || "0"), dst_port: parseInt(p.dst_endpoint.split(":").pop() || "0"), sequence: 0, ack_number: 0, data_offset: 0, flags: { syn: false, ack: false, fin: false, rst: false, psh: false, urg: false }, window: 0, checksum: 0, urgent_pointer: 0 } : null,
                    udp: p.protocol === "UDP" ? { src_port: parseInt(p.src_endpoint.split(":").pop() || "0"), dst_port: parseInt(p.dst_endpoint.split(":").pop() || "0"), length: 0, checksum: 0 } : null,
                    raw_bytes: p.raw_bytes,
                    payload: p.payload,
                    payload_hex: p.payload_hex,
                    payload_ascii: ascii,
                    capture_index: 0,
                  });
                });
                setView("capture");
              }}
              setStatusMessage={setStatusMessage}
            />
          )}
          {view === "replay" && (
            <ReplayView
              selectedPacket={selectedPacket}
              replayHistory={replayHistory}
              onRefreshHistory={loadSavedData}
              setStatusMessage={setStatusMessage}
            />
          )}
          {view === "sequences" && (
            <SequenceView
              sequences={sequences}
              savedPackets={savedPackets}
              onRefresh={loadSavedData}
              setStatusMessage={setStatusMessage}
            />
          )}
          {view === "fuzzer" && (
            <FuzzerView
              selectedPacket={selectedPacket}
              setStatusMessage={setStatusMessage}
            />
          )}
          {view === "stats" && (
            <StatsView packetCount={packets.length} />
          )}
        </main>
      </div>
    </div>
  );
}

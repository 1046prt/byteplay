import { useState, useEffect, useCallback, useRef } from "react";
import { commands } from "./commands";
import { POLL_INTERVAL_MS } from "./constants";
import type {
  CapturedPacket,
  SavedPacket,
  ReplayRecord,
  SavedSequence,
  ViewMode,
} from "./types";
import { Sidebar } from "./components/Sidebar";
import { Header } from "./components/Header";
import { CaptureView } from "./components/CaptureView";
import { LibraryView } from "./components/LibraryView";
import { ReplayView } from "./components/ReplayView";
import { SequenceView } from "./components/SequenceView";
import { FuzzerView } from "./components/FuzzerView";

export default function App() {
  const [view, setView] = useState<ViewMode>("capture");
  const [packets, setPackets] = useState<CapturedPacket[]>([]);
  const [selectedPacket, setSelectedPacket] = useState<CapturedPacket | null>(null);
  const [savedPackets, setSavedPackets] = useState<SavedPacket[]>([]);
  const [replayHistory, setReplayHistory] = useState<ReplayRecord[]>([]);
  const [sequences, setSequences] = useState<SavedSequence[]>([]);
  const [isCapturing, setIsCapturing] = useState(false);
  const [statusMessage, setStatusMessage] = useState("Ready");
  const pollRef = useRef<number | null>(null);

  const pollPackets = useCallback(async () => {
    try {
      const newPackets = await commands.pollPackets();
      if (newPackets.length > 0) {
        setPackets((prev) => [...prev, ...newPackets]);
      }
    } catch (e) {
      console.error("Poll error:", e);
    }
  }, []);

  useEffect(() => {
    if (isCapturing) {
      pollRef.current = window.setInterval(pollPackets, POLL_INTERVAL_MS);
    } else if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [isCapturing, pollPackets]);

  useEffect(() => {
    loadSavedData();
  }, []);

  const loadSavedData = async () => {
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
  };

  const handleClearPackets = async () => {
    await commands.clearPackets();
    setPackets([]);
    setSelectedPacket(null);
    setStatusMessage("Packets cleared");
  };

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
              setStatusMessage={setStatusMessage}
              setPackets={setPackets}
            />
          )}
          {view === "library" && (
            <LibraryView
              savedPackets={savedPackets}
              onRefresh={loadSavedData}
              onSelectPacket={(p) => {
                setSelectedPacket(null);
                setTimeout(() => {
                  setSelectedPacket({
                    id: p.id,
                    timestamp: p.timestamp,
                    interface: "library",
                    frame_length: p.raw_bytes.length,
                    ethernet: null,
                    ipv4: null,
                    ipv6: null,
                    tcp: null,
                    udp: null,
                    raw_bytes: p.raw_bytes,
                    payload: p.payload,
                    payload_hex: p.payload_hex,
                    payload_ascii: p.payload.map((b) => (b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : ".")).join(""),
                    capture_index: 0,
                  });
                }, 0);
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
        </main>
      </div>
    </div>
  );
}

import { invoke } from "@tauri-apps/api/core";
import type {
  CapturedPacket,
  CaptureConfig,
  ReplayConfig,
  ReplayResult,
  ReplayRecord,
  SavedPacket,
  SavedSequence,
  SequenceStep,
  FuzzConfig,
  FuzzResult,
  HexDiffEntry,
} from "./types";

export const commands = {
  listInterfaces: () => invoke<string[]>("list_interfaces"),

  startCapture: (config: CaptureConfig) =>
    invoke<string>("start_capture", { config }),
  stopCapture: () => invoke<string>("stop_capture"),
  pollPackets: () => invoke<CapturedPacket[]>("poll_packets"),
  getPacketById: (id: string) =>
    invoke<CapturedPacket | null>("get_packet_by_id", { id }),
  reparsePacket: (rawBytes: number[], interfaceName: string) =>
    invoke<CapturedPacket>("reparse_packet", { rawBytes, interface: interfaceName }),
  clearPackets: () => invoke<string>("clear_packets"),

  savePacket: (packetId: string, name: string, description: string, tags: string[]) =>
    invoke<SavedPacket>("save_packet", { packetId, name, description, tags }),
  getSavedPackets: () => invoke<SavedPacket[]>("get_saved_packets"),
  deleteSavedPacket: (id: string) =>
    invoke<string>("delete_saved_packet", { id }),
  updateSavedPacket: (id: string, name: string, description: string, tags: string[]) =>
    invoke<string>("update_saved_packet", { id, name, description, tags }),

  replayPacket: (data: number[], config: ReplayConfig) =>
    invoke<ReplayResult>("replay_packet", { data, config }),
  getReplayHistory: () => invoke<ReplayRecord[]>("get_replay_history"),

  executeReplaySequence: (steps: SequenceStep[], allowExternal: boolean) =>
    invoke<ReplayResult[]>("execute_replay_sequence", { steps, allowExternal }),
  runFuzzer: (config: FuzzConfig) =>
    invoke<FuzzResult[]>("run_fuzzer", { config }),

  saveSequence: (
    name: string,
    description: string,
    steps: SequenceStep[],
    tags: string[]
  ) => invoke<SavedSequence>("save_sequence", { name, description, steps, tags }),
  getSavedSequences: () => invoke<SavedSequence[]>("get_saved_sequences"),
  deleteSequence: (id: string) => invoke<string>("delete_sequence", { id }),

  exportPcap: (packetIds: string[], path: string) =>
    invoke<string>("export_pcap", { packetIds, path }),
  exportJson: (packetIds: string[], path: string) =>
    invoke<string>("export_json", { packetIds, path }),

  computeHexDiff: (original: number[], modified: number[]) =>
    invoke<HexDiffEntry[]>("compute_hex_diff", { original, modified }),
};

export interface EthernetInfo {
  src_mac: string;
  dst_mac: string;
  ether_type: string;
}

export interface IPv4Info {
  src_ip: string;
  dst_ip: string;
  version: number;
  ihl: number;
  dscp: number;
  ecn: number;
  total_length: number;
  identification: number;
  flags: number;
  fragment_offset: number;
  ttl: number;
  protocol: string;
  checksum: number;
}

export interface IPv6Info {
  src_ip: string;
  dst_ip: string;
  version: number;
  traffic_class: number;
  flow_label: number;
  payload_length: number;
  next_header: string;
  hop_limit: number;
}

export interface TcpFlags {
  syn: boolean;
  ack: boolean;
  fin: boolean;
  rst: boolean;
  psh: boolean;
  urg: boolean;
}

export interface TcpInfo {
  src_port: number;
  dst_port: number;
  sequence: number;
  ack_number: number;
  data_offset: number;
  flags: TcpFlags;
  window: number;
  checksum: number;
  urgent_pointer: number;
}

export interface UdpInfo {
  src_port: number;
  dst_port: number;
  length: number;
  checksum: number;
}

export interface CapturedPacket {
  id: string;
  timestamp: string;
  interface: string;
  frame_length: number;
  ethernet: EthernetInfo | null;
  ipv4: IPv4Info | null;
  ipv6: IPv6Info | null;
  tcp: TcpInfo | null;
  udp: UdpInfo | null;
  raw_bytes: number[];
  payload: number[];
  payload_hex: string;
  payload_ascii: string;
  capture_index: number;
}

export interface SavedPacket {
  id: string;
  name: string;
  description: string;
  timestamp: string;
  saved_at: string;
  protocol: string;
  src_endpoint: string;
  dst_endpoint: string;
  raw_bytes: number[];
  payload: number[];
  payload_hex: string;
  tags: string[];
}

export interface ReplayConfig {
  target_host: string;
  target_port: number;
  protocol: string;
  timeout_ms: number;
  allow_external: boolean;
}

export interface ReplayResult {
  success: boolean;
  bytes_sent: number;
  response: number[] | null;
  response_hex: string | null;
  error: string | null;
  duration_ms: number;
  target: string;
  timestamp: string;
}

export interface ReplayRecord {
  id: string;
  packet_id: string | null;
  packet_name: string | null;
  timestamp: string;
  target_host: string;
  target_port: number;
  protocol: string;
  bytes_sent: number;
  success: boolean;
  response_bytes: number[] | null;
  error: string | null;
  duration_ms: number;
}

export interface SequenceStep {
  name: string;
  target_host: string;
  target_port: number;
  protocol: string;
  data: number[];
  delay_ms: number;
  timeout_ms: number | null;
}

export interface SavedSequence {
  id: string;
  name: string;
  description: string;
  created_at: string;
  steps: SequenceStep[];
  tags: string[];
}

export interface FuzzConfig {
  target_host: string;
  target_port: number;
  protocol: string;
  base_payload: number[];
  iterations: number;
  mutation_rate: number;
  timeout_ms: number;
  allow_external: boolean;
}

export interface FuzzResult {
  iteration: number;
  mutated_payload: number[];
  replay_result: ReplayResult;
  mutations_applied: Mutation[];
}

export interface Mutation {
  offset: number;
  original: number;
  mutated: number;
}

export interface HexDiffEntry {
  offset: number;
  original: number | null;
  modified: number | null;
  changed: boolean;
}

export interface CaptureConfig {
  interface_name: string;
  bpf_filter: string;
  max_packets: number | null;
}

export type ViewMode = "capture" | "library" | "replay" | "sequences" | "fuzzer";

export interface CaptureStats {
  packetsPerSecond: number;
  bytesPerSecond: number;
  totalBytes: number;
  duration: number;
}

import { describe, it, expect, vi, beforeEach } from "vitest";
import { commands } from "./commands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const invokeMock = vi.mocked(invoke);

describe("commands", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("pollPackets passes the since cursor", async () => {
    invokeMock.mockResolvedValue({ packets: [], latest_seq: 42, dropped: 0 });
    await commands.pollPackets(7);
    expect(invokeMock).toHaveBeenCalledWith("poll_packets", { since: 7 });
  });

  it("pollPackets returns the PollBatch payload", async () => {
    const batch = {
      packets: [
        {
          id: "p1",
          seq: 1,
          timestamp: "t",
          interface: "eth0",
          frame_length: 64,
          ethernet: null,
          ipv4: null,
          ipv6: null,
          tcp: null,
          udp: null,
          raw_bytes: [],
          payload: [],
          payload_hex: "",
          payload_ascii: "",
          capture_index: 0,
        },
      ],
      latest_seq: 1,
      dropped: 2,
    };
    invokeMock.mockResolvedValue(batch);
    const result = await commands.pollPackets(0);
    expect(result.latest_seq).toBe(1);
    expect(result.dropped).toBe(2);
    expect(result.packets[0].id).toBe("p1");
  });

  it("cancelFuzzer invokes the cancel command", async () => {
    invokeMock.mockResolvedValue("ok");
    await commands.cancelFuzzer();
    expect(invokeMock).toHaveBeenCalledWith("cancel_fuzzer");
  });

  it("runFuzzer passes the fuzz config", async () => {
    invokeMock.mockResolvedValue([]);
    const config = {
      target_host: "127.0.0.1",
      target_port: 8080,
      protocol: "TCP",
      base_payload: [1, 2, 3],
      iterations: 10,
      mutation_rate: 0.1,
      timeout_ms: 3000,
      allow_external: false,
    };
    await commands.runFuzzer(config);
    expect(invokeMock).toHaveBeenCalledWith("run_fuzzer", { config });
  });
});

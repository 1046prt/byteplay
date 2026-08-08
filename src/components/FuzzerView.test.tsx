import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { FuzzerView } from "./FuzzerView";
import { commands } from "../commands";
import type { FuzzResult } from "../types";

vi.mock("../commands", () => ({
  commands: {
    runFuzzer: vi.fn(),
    cancelFuzzer: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

import { listen } from "@tauri-apps/api/event";
import type { Event } from "@tauri-apps/api/event";

const runFuzzerMock = vi.mocked(commands.runFuzzer);
const cancelFuzzerMock = vi.mocked(commands.cancelFuzzer);
const listenMock = vi.mocked(listen);

const makeResult = (iteration: number, success: boolean): FuzzResult => ({
  iteration,
  mutated_payload: [1, 2, 3],
  mutations_applied: [{ offset: 0, original: 1, mutated: 2 }],
  replay_result: {
    success,
    bytes_sent: 3,
    response: success ? [0x41] : null,
    response_hex: success ? "41" : null,
    error: success ? null : "connection refused",
    duration_ms: 5,
    target: "127.0.0.1:8080",
    timestamp: "t",
  },
});

const packet = {
  id: "p1",
  seq: 0,
  timestamp: "t",
  interface: "eth0",
  frame_length: 2,
  ethernet: null,
  ipv4: null,
  ipv6: null,
  tcp: null,
  udp: null,
  raw_bytes: [0x41, 0x42],
  payload: [0x41, 0x42],
  payload_hex: "4142",
  payload_ascii: "AB",
  capture_index: 0,
};

describe("FuzzerView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listenMock.mockResolvedValue(vi.fn());
  });

  it("renders the configuration panel with defaults", () => {
    render(<FuzzerView selectedPacket={packet} setStatusMessage={vi.fn()} />);
    expect(screen.getByText("Fuzzing Configuration")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /run 10 fuzz iterations/i })).toBeInTheDocument();
    expect(screen.getByDisplayValue("127.0.0.1")).toBeInTheDocument();
    expect(screen.getByDisplayValue("8080")).toBeInTheDocument();
  });

  it("runs the fuzzer and shows results", async () => {
    runFuzzerMock.mockResolvedValue([makeResult(1, true), makeResult(2, false)]);
    render(<FuzzerView selectedPacket={packet} setStatusMessage={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: /run 10 fuzz iterations/i }));

    await waitFor(() => {
      expect(runFuzzerMock).toHaveBeenCalledWith(
        expect.objectContaining({
          target_host: "127.0.0.1",
          target_port: 8080,
          iterations: 10,
          mutation_rate: 0.1,
          timeout_ms: 3000,
          allow_external: false,
        })
      );
    });

    await waitFor(() => {
      expect(screen.getByText("Complete")).toBeInTheDocument();
    });
    expect(screen.getByText("All (2)")).toBeInTheDocument();
    expect(screen.getByText("Errors (1)")).toBeInTheDocument();
    expect(screen.getByText("Responses (1)")).toBeInTheDocument();
  });

  it("filters results to errors only", async () => {
    runFuzzerMock.mockResolvedValue([makeResult(1, true), makeResult(2, false)]);
    render(<FuzzerView selectedPacket={packet} setStatusMessage={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: /run 10 fuzz iterations/i }));
    await waitFor(() => {
      expect(screen.getByText("Errors (1)")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("Errors (1)"));

    expect(screen.getByText("#2")).toBeInTheDocument();
    expect(screen.queryByText("#1")).not.toBeInTheDocument();
  });

  it("shows a cancel button while running and cancels on click", async () => {
    let resolveRun: (r: FuzzResult[]) => void = () => {};
    runFuzzerMock.mockImplementation(
      () =>
        new Promise<FuzzResult[]>((resolve) => {
          resolveRun = resolve;
        })
    );
    cancelFuzzerMock.mockResolvedValue("ok");

    render(<FuzzerView selectedPacket={packet} setStatusMessage={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /run 10 fuzz iterations/i }));

    const cancel = await screen.findByRole("button", { name: /cancel fuzz/i });
    fireEvent.click(cancel);
    expect(cancelFuzzerMock).toHaveBeenCalled();

    resolveRun([makeResult(1, true)]);
    await waitFor(() => {
      expect(screen.getByText("All (1)")).toBeInTheDocument();
    });
  });

  it("listens for fuzz-progress events and updates the progress bar", async () => {
    const progressHandler: {
      current: ((e: Event<{ done: number; total: number }>) => void) | null;
    } = {
      current: null,
    };
    listenMock.mockImplementation(
      (_event: string, handler: (e: Event<{ done: number; total: number }>) => void) => {
        progressHandler.current = handler;
        return Promise.resolve(vi.fn());
      }
    );
    let resolveRun: (r: FuzzResult[]) => void = () => {};
    runFuzzerMock.mockImplementation(
      () =>
        new Promise<FuzzResult[]>((resolve) => {
          resolveRun = resolve;
        })
    );

    render(<FuzzerView selectedPacket={packet} setStatusMessage={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /run 10 fuzz iterations/i }));

    await waitFor(() => {
      expect(progressHandler.current).not.toBeNull();
    });
    progressHandler.current!({ event: "fuzz-progress", id: 1, payload: { done: 5, total: 10 } });

    expect(await screen.findByText("5 / 10")).toBeInTheDocument();
    expect(screen.getByText("50%")).toBeInTheDocument();

    resolveRun([makeResult(1, true)]);
    await waitFor(() => {
      expect(screen.getByText("Complete")).toBeInTheDocument();
    });
  });
});

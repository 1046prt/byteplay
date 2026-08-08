import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { HexViewer } from "./HexViewer";

describe("HexViewer", () => {
  it("renders empty state when no bytes", () => {
    render(<HexViewer bytes={[]} />);
    expect(screen.getByText("(no data)")).toBeInTheDocument();
  });

  it("renders hex bytes and ascii for a full line", () => {
    const bytes = Array.from({ length: 16 }, (_, i) => i);
    render(<HexViewer bytes={bytes} />);
    expect(screen.getByText("00000000")).toBeInTheDocument();
    expect(screen.getByText("0f")).toBeInTheDocument();
    const ascii = bytes
      .map((b) => (b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : "."))
      .join("");
    expect(screen.getByText(ascii)).toBeInTheDocument();
  });

  it("wraps into multiple lines for long input", () => {
    const bytes = Array.from({ length: 20 }, (_, i) => i % 256);
    render(<HexViewer bytes={bytes} />);
    expect(screen.getByText("00000000")).toBeInTheDocument();
    expect(screen.getByText("00000010")).toBeInTheDocument();
  });

  it("escapes non-printable bytes in ascii column", () => {
    render(<HexViewer bytes={[0x00, 0x41, 0x7f]} />);
    expect(screen.getByText(".A.")).toBeInTheDocument();
  });
});

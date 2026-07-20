import { useMemo } from "react";

interface HexViewerProps {
  bytes: number[];
  editable?: boolean;
  onChange?: (bytes: number[]) => void;
}

const BYTES_PER_LINE = 16;

export function HexViewer({ bytes, editable = false, onChange }: HexViewerProps) {
  const lines = useMemo(() => {
    const result: Array<{
      offset: number;
      hex: Array<{ byte: number; index: number }>;
      ascii: string;
    }> = [];

    for (let i = 0; i < bytes.length; i += BYTES_PER_LINE) {
      const lineBytes: Array<{ byte: number; index: number }> = [];
      let ascii = "";
      for (let j = 0; j < BYTES_PER_LINE && i + j < bytes.length; j++) {
        const byte = bytes[i + j];
        lineBytes.push({ byte, index: i + j });
        ascii += byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : ".";
      }
      result.push({ offset: i, hex: lineBytes, ascii });
    }
    return result;
  }, [bytes]);

  if (bytes.length === 0) {
    return (
      <div className="text-xs text-gray-600 font-mono p-2">(no data)</div>
    );
  }

  return (
    <div className="font-mono text-[11px] leading-5 select-text">
      <div className="flex text-gray-600 mb-1 text-[10px]">
        <span className="w-[70px] shrink-0">Offset</span>
        <span className="flex gap-0">
          {Array.from({ length: BYTES_PER_LINE }, (_, i) => (
            <span key={i} className="w-[22px] text-center">
              {i.toString(16).padStart(2, "0").toUpperCase()}
            </span>
          ))}
        </span>
        <span className="ml-3 w-[130px] shrink-0">ASCII</span>
      </div>
      <div className="border-t border-[#1e293b]">
        {lines.map((line) => (
          <div key={line.offset} className="flex hover:bg-[#111827]">
            <span className="w-[70px] text-gray-600 shrink-0">
              {line.offset.toString(16).padStart(8, "0")}
            </span>
            <span className="flex gap-0">
              {line.hex.map((entry) => (
                <span
                  key={entry.index}
                  className="w-[22px] text-center text-gray-400"
                >
                  {entry.byte.toString(16).padStart(2, "0")}
                </span>
              ))}
              {line.hex.length < BYTES_PER_LINE &&
                Array.from({ length: BYTES_PER_LINE - line.hex.length }, (_, i) => (
                  <span key={`pad-${i}`} className="w-[22px] text-center text-gray-800">
                    ·
                  </span>
                ))}
            </span>
            <span className="ml-3 w-[130px] text-gray-500 shrink-0 whitespace-pre">
              {line.ascii}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

# byteplay

**Multi-platform network packet analyzer & replayer** — capture, edit, and replay raw TCP/UDP packets to test server resilience.

---

## Why byteplay?

| Tool | Limitation | byteplay's Edge |
|---|---|---|
| Wireshark | Capture/analyze only, no edit-and-resend | Full round-trip: capture → edit → replay |
| Burp Suite | HTTP/HTTPS only | Protocol-agnostic (raw TCP/UDP, any payload) |
| Scapy (scriptable) | No GUI, steep learning curve | Clean GUI, zero scripting required |
| Postman | Application-layer (REST) only | Works below the application layer |
| hping3/nc | CLI only, no persistent packet library | Saved packet library + diffing + replay history |

**Key differentiators:**
- **Byte-level hex diffing** — visual diff between original and edited packets
- **Replay sequences** — chain packets into scripted session replays
- **Mutation fuzzing** — flip random bytes, log crashes/timeouts
- **Cross-platform native performance** — Rust capture core via Tauri
- **Local-first, zero telemetry** — no data leaves your machine

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│               Frontend (React + TypeScript)      │
│   - Virtualized packet list                      │
│   - Hex/ASCII dual-pane viewer                   │
│   - Replay console + response viewer             │
│   - Sequence editor + fuzzer dashboard           │
└───────────────────▲──────────────────┬──────────┘
                    │ Tauri IPC         │
┌───────────────────┴──────────────────▼──────────┐
│             Capture Engine (Rust)                │
│   pnet: raw socket capture + protocol parsing    │
│   socket2: TCP/UDP raw replay                   │
│   rusqlite: local packet library + history       │
│   Multi-threaded ring buffer, non-blocking UI    │
└─────────────────────────────────────────────────┘
```

## Tech Stack

| Layer | Technology | Why |
|---|---|---|
| Capture core | Rust + `pnet` | Memory-safe raw socket access, near-C performance |
| App shell | **Tauri 2** | Rust backend + WebView frontend, small binary |
| Frontend | React 19 + TypeScript + Tailwind v4 | Fast iteration, virtualized lists |
| Hex viewer | Custom component | Performant dual-pane rendering |
| Storage | SQLite via `rusqlite` | Simple, portable, no server |
| Replay | `socket2` crate | Precise TCP/UDP control |

---

## Project Structure

```
byteplay/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── capture.rs      # Packet capture engine (pnet)
│   │   ├── parser.rs       # Ethernet/IP/TCP/UDP parsing
│   │   ├── replay.rs       # TCP/UDP replay + fuzzing
│   │   ├── storage.rs      # SQLite + PCAP/JSON export
│   │   ├── lib.rs          # Tauri commands & state
│   │   └── main.rs         # Entry point
│   └── Cargo.toml
├── src/                    # React frontend
│   ├── components/
│   │   ├── Sidebar.tsx
│   │   ├── Header.tsx
│   │   ├── CaptureView.tsx
│   │   ├── PacketList.tsx
│   │   ├── PacketDetail.tsx
│   │   ├── HexViewer.tsx
│   │   ├── LibraryView.tsx
│   │   ├── ReplayView.tsx
│   │   ├── SequenceView.tsx
│   │   └── FuzzerView.tsx
│   ├── App.tsx
│   ├── main.tsx
│   ├── commands.ts         # Tauri IPC wrappers
│   ├── types.ts            # Shared TypeScript types
│   └── style.css
├── index.html
├── vite.config.ts
├── LICENSE
└── README.md
```

---

## Setup & Development

### Prerequisites

- **Node.js** 18+ and npm
- **Rust** toolchain (via [rustup](https://rustup.rs/))
- **Platform-specific requirements:**
  - **Windows:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "C++ build tools" workload + [Npcap](https://npcap.com/)
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`) + [Homebrew libpcap](https://formulae.brew.sh/formula/libpcap) (`brew install libpcap`)
  - **Linux:** `sudo apt install libpcap-dev`

### Install Dependencies

```bash
# Frontend
npm install

# Rust backend (from src-tauri/)
cd src-tauri && cargo build
```

### Development

```bash
# Start both frontend dev server and Tauri app
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

---

## Security & Ethics

This tool touches raw sockets and packet crafting. Please note:

- **Default bind**: targets `localhost` / private network ranges only
- **External targets**: require explicit opt-in (`Allow external targets` checkbox)
- **Audit logging**: every replay action is logged locally with timestamp, target, and payload hash
- **License**: MIT — "For use only against systems you own or have explicit authorization to test"

---

## Known Limitations

- **Npcap required on Windows**: packet capture needs Npcap (or WinPcap) installed
- **No TLS decryption**: this tool operates at the raw socket level; TLS is opaque
- **No pcap injection**: capture uses pnet's datalink channel, not raw pcap injection
- **Single interface capture**: one capture session at a time

---

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/1046prt/byteplay).

---

## License

[MIT](LICENSE)

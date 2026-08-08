# byteplay

**Multi-platform network packet analyzer & replayer** — capture, edit, and replay raw TCP/UDP packets to test server resilience.

---

## Why byteplay?

| Tool               | Limitation                               | byteplay's Edge                                 |
| ------------------ | ---------------------------------------- | ----------------------------------------------- |
| Wireshark          | Capture/analyze only, no edit-and-resend | Full round-trip: capture → edit → replay        |
| Burp Suite         | HTTP/HTTPS only                          | Protocol-agnostic (raw TCP/UDP, any payload)    |
| Scapy (scriptable) | No GUI, steep learning curve             | Clean GUI, zero scripting required              |
| Postman            | Application-layer (REST) only            | Works below the application layer               |
| hping3/nc          | CLI only, no persistent packet library   | Saved packet library + diffing + replay history |

**Key differentiators:**

- **Byte-level hex diffing** — visual diff between original and edited packets
- **Replay sequences** — chain packets into scripted session replays
- **Mutation fuzzing** — deterministic seeded fuzzer with live progress + cancel
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

| Layer        | Technology                          | Why                                               |
| ------------ | ----------------------------------- | ------------------------------------------------- |
| Capture core | Rust + `pnet`                       | Memory-safe raw socket access, near-C performance |
| App shell    | **Tauri 2**                         | Rust backend + WebView frontend, small binary     |
| Frontend     | React 19 + TypeScript + Tailwind v4 | Fast iteration, virtualized lists                 |
| Hex viewer   | Custom component                    | Performant dual-pane rendering                    |
| Storage      | SQLite via `rusqlite`               | WAL mode, busy-timeout, bounded history (5k)      |
| Replay       | `socket2` crate                     | Precise TCP/UDP control                           |

---

## Project Structure

```
byteplay/
├── .github/workflows/     # CI + release pipelines
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── capture.rs     # Packet capture engine (pnet) + delta polling
│   │   ├── parser.rs      # Ethernet/IP/TCP/UDP parsing (+ raw IP linktype)
│   │   ├── replay.rs      # TCP/UDP replay + fuzzing (cancel + progress)
│   │   ├── storage.rs     # SQLite (WAL) + PCAP/JSON export
│   │   ├── lib.rs         # Tauri commands & state
│   │   └── main.rs        # Entry point
│   └── Cargo.toml
├── src/                   # React frontend
│   ├── components/
│   │   ├── Sidebar.tsx / Header.tsx
│   │   ├── CaptureView.tsx / PacketList.tsx / PacketDetail.tsx
│   │   ├── HexViewer.tsx
│   │   ├── LibraryView.tsx
│   │   ├── ReplayView.tsx
│   │   ├── SequenceView.tsx
│   │   ├── FuzzerView.tsx
│   │   ├── StatsView.tsx
│   │   └── Toast.tsx / ContextMenu.tsx / ErrorBoundary.tsx
│   ├── *.test.ts(x)       # Vitest unit tests
│   ├── App.tsx
│   ├── main.tsx
│   ├── commands.ts        # Tauri IPC wrappers
│   ├── types.ts           # Shared TypeScript types
│   └── style.css
├── index.html
├── vite.config.ts
├── vitest.config.ts
├── eslint.config.js
├── LICENSE
└── README.md
```

---

## Setup & Development

### Prerequisites

- **Node.js** 20+ and npm
- **Rust** toolchain (via [rustup](https://rustup.rs/))
- **Platform-specific requirements:**
  - **Windows:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "C++ build tools" workload + [Npcap](https://npcap.com/). Building from source also requires the [Npcap SDK](https://npcap.com/dist/npcap-sdk-1.13.zip), extracted to `src-tauri/npcap-sdk/`
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

## Testing & Quality

```bash
# Backend (from src-tauri/)
cargo test               # 37 unit + integration tests
cargo clippy --all-targets -- -D warnings   # zero-warning lint gate
cargo fmt --check        # formatting gate

# Frontend
npm test                 # 13 Vitest unit tests (jsdom + testing-library)
npm run lint             # ESLint (typescript-eslint + react-hooks)
npm run format:check     # Prettier check
npx tsc --noEmit         # Type checking
```

Coverage areas: packet parsing (Ethernet/IPv4/IPv6/TCP/UDP incl. raw-IP linktype), capture ring buffer + delta polling, fuzzer clamping/cancellation/progress determinism, SQLite hardening (WAL, busy-timeout, history pruning, poisoned-mutex recovery), and frontend views (hex viewer, fuzzer controls, command IPC contracts).

CI runs all of the above on push/PR across Linux, Windows, and macOS (`.github/workflows/ci.yml`); tag pushes (`v*`) trigger installer builds (`.github/workflows/release.yml`).

---

## Security & Ethics

This tool touches raw sockets and packet crafting. Please note:

- **Default bind**: targets `localhost` / private network ranges only
- **External targets**: require explicit opt-in (`Allow external targets` checkbox)
- **Audit logging**: every replay action is logged locally with timestamp, target, and payload hash
- **License**: MIT — "For use only against systems you own or have explicit authorization to test"

---

## Known Limitations

- **Npcap required on Windows**: packet capture needs Npcap (or WinPcap) installed; the SDK is only needed when compiling
- **No TLS decryption**: this tool operates at the raw socket level; TLS is opaque
- **No pcap injection**: capture uses pnet's datalink channel, not raw pcap injection
- **Single interface capture**: one capture session at a time
- **Capture buffer**: in-memory ring buffer (50,000 packets) — older packets are evicted while capturing; use the packet library to persist them

---

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/1046prt/byteplay).

---

## License

[MIT](LICENSE)

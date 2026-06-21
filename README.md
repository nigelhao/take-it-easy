# pitch-shifter

A pitch shifting program with a simple web app client to control the pitch, intended for home karaoke use.

Two flavours of the same UI:

- **Linux native daemon** (recommended on Linux) — Rust binary that opens JACK/PipeWire ports directly. Sub-15 ms mic-to-speaker latency.

## Linux native daemon

### Requirements

- PipeWire with the `pipewire-jack` shim (default on modern Arch / Fedora / Ubuntu).
- Rust toolchain.
- `libjack` development headers (provided by `pipewire-jack` on PipeWire systems, or `jack2`).

### Build

```
cargo build --release
```

Binary: `target/release/pitch-shifter`. Static assets must be reachable as a `static/` directory next to the binary, or set via `STATIC_DIR=…`.

### Run

```
/usr/bin/pw-jack ./target/release/pitch-shifter
```

Then open <http://localhost:8080>. The daemon auto-connects to the system mic and speakers; if it doesn't, use `qpwgraph` or `pw-link` to wire `pitch-shifter:input` and `pitch-shifter:output`.

Environment overrides:

- `BIND_ADDR` — listen address (default `127.0.0.1:8080`).
- `STATIC_DIR` — path to the `static/` directory (default: next to the binary, falling back to CWD).
- `RUST_LOG` — log level filter (default `info`).

### How it works

- JACK client at 48 kHz, 128-sample quantum (`PIPEWIRE_LATENCY=128/48000` is set automatically).
- Pitch shift runs on the realtime audio thread: crossfading dual-tap delay line with a 1024-sample ring buffer (~10 ms algorithmic latency when shifting; **bypassed at 0 semitones** for sample-accurate passthrough).
- Web UI talks to the daemon over a local WebSocket (`/ws`). Semitone updates are stored in an `AtomicI32` that the audio thread reads lock-free.

### Latency budget (expected)

|                          | At 0 semitones | When shifting |
| ------------------------ | -------------- | ------------- |
| PipeWire I/O quantum × 2 | ~5 ms          | ~5 ms         |
| Algorithm                | 0 ms (bypass)  | ~10 ms        |
| **Total RTT**            | **~5 ms**      | **~15 ms**    |

Driver/hardware adds 1–3 ms on top for typical USB interfaces.

### Verify

- `pw-top` — confirm the `pitch-shifter` node shows `quantum 128` at `rate 48000`. If it shows 1024, the latency hint was ignored — check the daemon's logs.
- `pw-top` `ERR` column should stay 0. If xruns appear, raise quantum (e.g. `PIPEWIRE_LATENCY=256/48000 ./pitch-shifter`) or check that PipeWire's RT scheduling is granted (the daemon prints its negotiated buffer size at startup).
- Roundtrip measurement: loopback output to input with a cable, clap into the mic, record with Audacity, measure the gap.

## Browser-only

The original implementation. Open `index.html` over a local HTTP server (mic access requires a secure context — `file://` won't work):

```
python3 -m http.server 8000
```

Then visit <http://localhost:8000>.

Same UI: `♭` lower, `♯` raise, `Reset` to 0, range ±12. Keyboard `↑ ↓ + - 0 space` also work.

## Files

- `src/` — Rust daemon (shifter, JACK audio, axum HTTP/WS server).
- `static/` — daemon's web UI (talks to daemon over WebSocket).

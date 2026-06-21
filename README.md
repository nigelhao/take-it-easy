# pitch-shifter

A real-time pitch shifting application with ultra-low latency, designed for home karaoke use. Features a clean web interface to control pitch shifting from your microphone input stream.

## Why This Project?

This project was created to enhance the home karaoke experience by leveraging **Apple Music Sing on Apple TV**, which uses AI to remove vocals in real-time. The goal is to provide seamless key control through a nice UI accessible from Apple TV, while integrating professional audio equipment for superior sound quality.

### The Complete Setup

The system combines:

- **Apple Music Sing**: AI-powered real-time vocal removal on Apple TV
- **Computer** that runs **take-it-easy**: Real-time pitch shifting with ultra-low latency
- **Yamaha MG06X Mixing Console**: Professional audio mixing with reverb for microphones
- **SSS1700C1 USB Sound Card**: Low-latency digital audio interface with TOSLINK support

This setup allows singers to:

1. Choose any song on Apple TV with vocal removal
2. Adjust the key to match their vocal range via the web UI
3. Mix their microphone input with professional reverb effects
4. Enjoy studio-quality audio output through speakers

### Audio Signal Flow

```
Apple TV (Music Sing)
    │
    │ HDMI (Audio + Video)
    ▼
TV Display
    │
    │ TOSLINK (Digital Audio)
    ▼
SSS1700C1 USB Sound Card (connected to host)
    │
    │ USB (Digital Audio Input)
    ▼
Host running take-it-easy
    │
    │ (Real-time pitch shifting)
    │ (Web UI for key control) ◄── User interact with website
    │
    │ USB (Digital Audio Output)
    ▼
SSS1700C1 USB Sound Card (connected to same host)
    │
    │ TOSLINK (Digital Audio)
    ▼
TOSLINK-supported DAC
    │
    │ Analog Audio
    ▼
Yamaha MG06X Mixing Console ◄── Microphones
    │
    │ Mixed Analog Audio
    ▼
Amplifier
    │
    ▼
Speakers
```

**Key Benefits:**

- **Digital audio path** from Apple TV to PC maintains pristine quality
- **Ultra-low latency** pitch shifting (~15ms) feels instantaneous
- **Professional mixing** with Yamaha console adds studio-quality reverb
- **Flexible key control** via web interface accessible from any device
- **No vocal bleed** thanks to Apple Music Sing's AI separation

## Features

- **Ultra-low latency**: Sub-15ms mic-to-speaker round-trip when pitch shifting, ~5ms in bypass mode
- **Real-time processing**: Runs on JACK/PipeWire's realtime audio thread
- **Smart bypass**: Zero-latency passthrough at 0 semitones (sample-accurate)
- **Web interface**: Simple, responsive UI accessible from any browser
- **Keyboard shortcuts**: Control pitch without touching the mouse
- **Range**: ±12 semitones (one octave up or down)

## Architecture

### Audio Processing

- **JACK client** running at 48 kHz with 128-sample quantum
- **Crossfading dual-tap delay line** pitch shifter with 1024-sample ring buffer
- **Lock-free communication** between web server and audio thread via `AtomicI32`
- **Automatic port connection** to system microphone and speakers

### Web Stack

- **Backend**: Rust with Axum (HTTP + WebSocket server)
- **Frontend**: Vanilla JavaScript with WebSocket for real-time control
- **Static assets**: Served from `static/` directory

## Requirements

### Software

- **PipeWire** with `pipewire-jack` shim (default on modern Arch, Fedora, Ubuntu)
- **Rust toolchain** (1.70+)
- **libjack development headers**:
    - On PipeWire systems: provided by `pipewire-jack`
    - On JACK systems: install `jack2` or `libjack-dev`

### Hardware

This project was developed and tested with the **SSS1700C1 USB Sound Card** (Type-C interface), which provides excellent low-latency performance for karaoke applications.

- Product: [SSS1700C1 USB Sound Card on AliExpress](https://www.aliexpress.com/item/1005010196963412.html)
- Features: Type-C interface, low latency, suitable for real-time audio processing
- Note: Other USB audio interfaces should work as well, but latency may vary

## Installation

### Build from Source

```bash
cargo build --release
```

The binary will be located at `target/release/pitch-shifter`.

**Note**: Static assets must be accessible as a `static/` directory next to the binary, or specify a custom path via the `STATIC_DIR` environment variable.

## Usage

### Starting the Server

```bash
pw-jack ./target/release/pitch-shifter
```

Then open your browser to <http://localhost:8080>

The daemon automatically connects to your system microphone and speakers. If auto-connection fails, manually wire the ports using `qpwgraph` or `pw-link`:

- Input: `pitch-shifter:input` ← your microphone
- Output: `pitch-shifter:output` → your speakers

### Web Interface

The web UI provides three main controls:

- **♭ (Flat)**: Lower pitch by 1 semitone
- **Reset**: Return to 0 semitones (bypass mode)
- **♯ (Sharp)**: Raise pitch by 1 semitone

**Keyboard shortcuts**:

- `↑` or `+`: Raise pitch
- `↓` or `-`: Lower pitch
- `0` or `Space`: Reset to 0

The display shows the current pitch shift in semitones, with a connection indicator showing WebSocket status.

### Environment Variables

Customize behavior with these environment variables:

| Variable           | Default          | Description                                           |
| ------------------ | ---------------- | ----------------------------------------------------- |
| `BIND_ADDR`        | `127.0.0.1:8080` | Server listen address and port                        |
| `STATIC_DIR`       | `./static`       | Path to static assets directory                       |
| `RUST_LOG`         | `info`           | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `PIPEWIRE_LATENCY` | `128/48000`      | PipeWire quantum/rate (set automatically)             |

**Examples**:

```bash
# Listen on all interfaces
BIND_ADDR=0.0.0.0:8080 pw-jack ./target/release/pitch-shifter

# Enable debug logging
RUST_LOG=debug pw-jack ./target/release/pitch-shifter

# Custom static directory
STATIC_DIR=/path/to/static pw-jack ./target/release/pitch-shifter

# Higher latency for stability on slower systems
PIPEWIRE_LATENCY=256/48000 pw-jack ./target/release/pitch-shifter
```

## Technical Details

### How It Works

1. **JACK Integration**: Connects to PipeWire/JACK at 48 kHz with a 128-sample quantum (~2.7ms per buffer)
2. **Pitch Shifting Algorithm**:
    - Two read pointers traverse a 1024-sample ring buffer at `ratio = 2^(semitones/12)`
    - Hann window crossfading prevents clicks as pointers wrap around
    - Linear interpolation for smooth sub-sample reading
3. **Bypass Mode**: At 0 semitones, the algorithm is completely bypassed for zero-latency passthrough
4. **Real-time Safety**: All audio processing runs lock-free on the JACK realtime thread

### Latency Budget

Expected round-trip latency at 48 kHz, 128-sample quantum:

| Component                  | At 0 semitones | When shifting |
| -------------------------- | -------------- | ------------- |
| PipeWire I/O (quantum × 2) | ~5 ms          | ~5 ms         |
| Pitch shift algorithm      | 0 ms (bypass)  | ~10 ms        |
| **Total RTT**              | **~5 ms**      | **~15 ms**    |

_Note: USB audio interfaces typically add 1–3ms of additional latency._

### Performance Verification

#### Check Quantum Settings

```bash
pw-top
```

Verify the `pitch-shifter` node shows:

- `quantum: 128`
- `rate: 48000`

If it shows `quantum: 1024`, the latency hint was ignored. Check the daemon's startup logs for warnings.

#### Monitor for Xruns

In `pw-top`, watch the `ERR` column for the `pitch-shifter` node. It should stay at 0.

If xruns (buffer underruns) occur:

1. **Increase quantum**: `PIPEWIRE_LATENCY=256/48000 ./pitch-shifter`
2. **Check RT scheduling**: Ensure PipeWire has realtime permissions
3. **Review system load**: Close unnecessary applications

The daemon logs its negotiated buffer size at startup for verification.

## Project Structure

```
.
├── src/
│   ├── main.rs      # Entry point, initialization, signal handling
│   ├── audio.rs     # JACK client, audio thread, port management
│   ├── shifter.rs   # Pitch shifting algorithm implementation
│   └── server.rs    # Axum HTTP/WebSocket server
├── static/
│   ├── index.html   # Web UI markup and styles
│   └── app.js       # WebSocket client and UI logic
├── Cargo.toml       # Rust dependencies and build config
└── README.md        # This file
```

## Troubleshooting

### "JACK client connect failed"

- Ensure PipeWire is running: `systemctl --user status pipewire`
- Check if `pipewire-jack` is installed: `ldconfig -p | grep libjack`

### High Latency / Quantum Not 128

- The daemon sets `PIPEWIRE_LATENCY=128/48000` automatically
- If ignored, check PipeWire configuration in `~/.config/pipewire/`
- Some systems may enforce minimum quantum limits

### No Audio / Ports Not Connected

- List available ports: `pw-link -l`
- Manually connect:
    ```bash
    pw-link <your-mic-port> pitch-shifter:input
    pw-link pitch-shifter:output <your-speaker-port>
    ```
- Use `qpwgraph` for a visual patchbay

### Audio Artifacts / Glitches

- Increase buffer size: `PIPEWIRE_LATENCY=256/48000`
- Check CPU usage: `top` or `htop`
- Extreme pitch shifts (±12 semitones) may have more artifacts due to the algorithm's design

### WebSocket Connection Failed

- Check firewall settings if accessing from another device
- Verify the server is listening: `netstat -tlnp | grep 8080`
- Check browser console for error messages

## Development

### Running in Debug Mode

```bash
RUST_LOG=debug cargo run
```

### Running Tests

```bash
cargo test
```

Tests include:

- Bypass mode verification (sample-accurate passthrough)
- Stability checks across pitch shift range
- Finite output validation

### Building for Production

The release profile is optimized for size and performance:

- Link-time optimization (LTO)
- Single codegen unit
- Panic abort (smaller binary)
- Debug symbols stripped

## License

See project repository for license information.

## Credits

Pitch shifting algorithm ported from `pitch-worklet.js`, adapted for lower latency (1024-sample buffer vs. 2048) to achieve ~10ms algorithmic latency suitable for live vocal processing.

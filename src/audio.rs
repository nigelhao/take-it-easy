// JACK client running the pitch shifter on the realtime audio thread.
//
// On Linux with PipeWire, libjack is provided by `pipewire-jack` — the JACK
// API is a thin shim over the same PipeWire transport, so latency matches
// what you'd get from the native PipeWire client API.
//
// `PIPEWIRE_LATENCY=128/48000` is set before connecting so PipeWire creates
// our node at a 128-sample quantum at 48 kHz regardless of the system default
// (typically 1024 — the high-latency trap from the plan).

use anyhow::{anyhow, Result};
use jack::{contrib::ClosureProcessHandler, AudioIn, AudioOut, Client, ClientOptions, Control, PortFlags};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use crate::shifter::PitchShifter;

pub struct AudioInfo {
    pub sample_rate: u32,
    pub buffer_size: u32,
}

/// Activates the JACK client. The returned handle keeps the audio thread
/// running; dropping it deactivates the client.
pub fn start(semitones: Arc<AtomicI32>) -> Result<(impl Drop + Send, AudioInfo)> {
    std::env::set_var("PIPEWIRE_LATENCY", "128/48000");

    let (client, status) = Client::new("pitch-shifter", ClientOptions::NO_START_SERVER)
        .map_err(|e| anyhow!("JACK client connect failed: {e:?}"))?;
    tracing::info!(?status, "JACK client connected");

    let sample_rate = client.sample_rate() as u32;
    let buffer_size = client.buffer_size();
    tracing::info!(sample_rate, buffer_size, "JACK negotiated");

    if sample_rate != 48000 {
        tracing::warn!(
            sample_rate,
            "server is not at 48 kHz — pitch shift still works but the latency target assumes 48 kHz"
        );
    }

    let in_port = client.register_port("input", AudioIn::default())?;
    let mut out_port = client.register_port("output", AudioOut::default())?;

    let in_port_name = in_port.name()?;
    let out_port_name = out_port.name()?;

    let mut shifter = PitchShifter::new();
    let semitones_for_audio = semitones.clone();

    let handler = ClosureProcessHandler::new(move |_c: &Client, ps: &jack::ProcessScope| {
        let input = in_port.as_slice(ps);
        let output = out_port.as_mut_slice(ps);
        let semis = semitones_for_audio.load(Ordering::Relaxed).clamp(-12, 12);
        shifter.process(input, output, semis);
        Control::Continue
    });

    let active = client
        .activate_async((), handler)
        .map_err(|e| anyhow!("JACK activate failed: {e:?}"))?;

    if let Err(e) = autoconnect(active.as_client(), &in_port_name, &out_port_name) {
        tracing::warn!(
            "autoconnect failed: {e}. Use `qpwgraph` or `pw-link` to wire the ports manually."
        );
    }

    Ok((
        active,
        AudioInfo {
            sample_rate,
            buffer_size,
        },
    ))
}

/// Best-effort: connect the first physical capture port to our input, and our
/// output to all physical playback ports (so a mono shift goes to both speakers).
fn autoconnect(client: &Client, in_port: &str, out_port: &str) -> Result<()> {
    let mic_candidates = client.ports(
        None,
        Some("32 bit float mono audio"),
        PortFlags::IS_PHYSICAL | PortFlags::IS_OUTPUT,
    );
    let speaker_candidates = client.ports(
        None,
        Some("32 bit float mono audio"),
        PortFlags::IS_PHYSICAL | PortFlags::IS_INPUT,
    );

    if let Some(mic) = mic_candidates.first() {
        tracing::info!("connecting mic {mic} → {in_port}");
        client.connect_ports_by_name(mic, in_port)?;
    } else {
        tracing::warn!("no physical capture port found");
        std::process::exit(1); /// to force systemctl to restart the service if no input is found
    }

    if speaker_candidates.is_empty() {
        tracing::warn!("no physical playback port found");
    }
    for speaker in speaker_candidates.iter().take(2) {
        tracing::info!("connecting {out_port} → {speaker}");
        client.connect_ports_by_name(out_port, speaker)?;
    }

    Ok(())
}

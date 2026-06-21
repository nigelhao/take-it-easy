mod audio;
mod server;
mod shifter;

use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let semitones = Arc::new(AtomicI32::new(0));

    // JACK client lives until we drop `_active`. Holds the realtime audio thread.
    let (_active, info) = audio::start(semitones.clone())?;

    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    let static_dir: PathBuf = std::env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let exe = std::env::current_exe().ok();
            let beside = exe
                .as_ref()
                .and_then(|p| p.parent())
                .map(|p| p.join("static"));
            if let Some(p) = beside.as_ref() {
                if p.is_dir() {
                    return p.clone();
                }
            }
            PathBuf::from("static")
        });

    tracing::info!(?static_dir, "serving static assets");

    let server_handle = tokio::spawn(server::run(
        semitones.clone(),
        info.sample_rate,
        info.buffer_size,
        static_dir,
        addr,
    ));

    tokio::select! {
        res = server_handle => {
            match res {
                Ok(Ok(())) => tracing::info!("server exited"),
                Ok(Err(e)) => tracing::error!("server error: {e:?}"),
                Err(e) => tracing::error!("server task join error: {e:?}"),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("ctrl-c received, shutting down");
        }
    }

    Ok(())
}

use crate::tts::AudioData;
use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

/// Play audio bytes via the default output device.
/// Blocks until playback completes.
pub fn play_audio(audio: &AudioData) -> Result<()> {
    let (_stream, stream_handle) =
        OutputStream::try_default().context("Failed to open audio output device")?;
    let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;

    let cursor = Cursor::new(audio.bytes.clone());
    let source = Decoder::new(cursor).context("Failed to decode audio data")?;

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

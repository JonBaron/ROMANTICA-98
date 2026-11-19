use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use xmrs::prelude::*;
use xmrsplayer::prelude::*;

/// Spiller XM-modulen så lenge verdien lever.
pub struct Music {
    _stream: cpal::Stream,
}

pub fn play(xm: &[u8]) -> Result<Music, String> {
    let module = Module::load(xm).map_err(|e| format!("XMLoad Error: {e:?}"))?;
    let module: &'static Module = Box::leak(Box::new(module));

    let device = cpal::default_host()
        .default_output_device()
        .ok_or("No device")?;
    let config = device
        .default_output_config()
        .map_err(|e| format!("No sound: {e}"))?;
    let channels = config.channels() as usize;
    let sample_rate: u32 = config.sample_rate();

    let player = Arc::new(Mutex::new(XmrsPlayer::new(module, sample_rate, 0)));
    let stream = device
        .build_output_stream(
            &config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut player = player.lock().unwrap();
                for frame in data.chunks_mut(channels) {
                    let l = player.next().unwrap_or(0) as f32 / i16::MAX as f32;
                    let r = player.next().unwrap_or(0) as f32 / i16::MAX as f32;
                    match frame.len() {
                        1 => frame[0] = (l + r) * 0.5,
                        _ => {
                            frame[0] = l;
                            frame[1] = r;
                            frame[2..].fill(0.0);
                        }
                    }
                }
            },
            |e| eprintln!("lydfeil: {e}"),
            None,
        )
        .map_err(|e| format!("Can't open sound stream: {e}"))?;
    stream.play().map_err(|e| format!("Can't start sound: {e}"))?;
    Ok(Music { _stream: stream })
}

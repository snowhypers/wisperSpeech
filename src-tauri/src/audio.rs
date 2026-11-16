use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter};

pub struct AudioRecorder {
    is_recording: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: Arc<Mutex<u32>>,
    stream: Option<cpal::Stream>,
}

// cpal::Stream is not Send on all platforms; wrap in a dedicated thread or manage safely.
unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Arc::new(Mutex::new(16000)),
            stream: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn start(&mut self, app: AppHandle) -> Result<()> {
        if self.is_active() {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No default microphone input device found"))?;

        let config = device.default_input_config()?;
        let channels = config.channels() as usize;
        let input_rate = config.sample_rate().0;

        {
            let mut s = self.samples.lock().unwrap();
            s.clear();
            *self.sample_rate.lock().unwrap() = input_rate;
        }

        self.is_recording.store(true, Ordering::SeqCst);
        let samples_ref = self.samples.clone();
        let flag_ref = self.is_recording.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if !flag_ref.load(Ordering::SeqCst) {
                        return;
                    }
                    let mut mono = Vec::with_capacity(data.len() / channels.max(1));
                    let mut sum_sq = 0.0f32;
                    for frame in data.chunks(channels.max(1)) {
                        let avg = frame.iter().sum::<f32>() / (channels.max(1) as f32);
                        sum_sq += avg * avg;
                        mono.push(avg);
                    }
                    if !mono.is_empty() {
                        let rms = (sum_sq / mono.len() as f32).sqrt() * 4.5;
                        let _ = app.emit("audio-level", rms.min(1.0));
                        samples_ref.lock().unwrap().extend_from_slice(&mono);
                    }
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if !flag_ref.load(Ordering::SeqCst) {
                        return;
                    }
                    let mut mono = Vec::with_capacity(data.len() / channels.max(1));
                    let mut sum_sq = 0.0f32;
                    for frame in data.chunks(channels.max(1)) {
                        let avg = frame
                            .iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .sum::<f32>()
                            / (channels.max(1) as f32);
                        sum_sq += avg * avg;
                        mono.push(avg);
                    }
                    if !mono.is_empty() {
                        let rms = (sum_sq / mono.len() as f32).sqrt() * 4.5;
                        let _ = app.emit("audio-level", rms.min(1.0));
                        samples_ref.lock().unwrap().extend_from_slice(&mono);
                    }
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )?,
            _ => return Err(anyhow!("Unsupported microphone sample format")),
        };

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    /// Stops recording, resamples to 16 kHz mono, and writes a WAV file for Cactus Whistle.
    pub fn stop_to_wav(&mut self, out_path: PathBuf) -> Result<()> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream.take();

        let raw = {
            let mut guard = self.samples.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        let src_rate = *self.sample_rate.lock().unwrap();

        if raw.is_empty() {
            return Err(anyhow!("No audio captured"));
        }

        let resampled = resample_linear(&raw, src_rate, 16000);

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&out_path, spec)?;
        for sample in resampled {
            let clamped = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(clamped)?;
        }
        writer.finalize()?;
        Ok(())
    }
}

fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = src_rate as f64 / dst_rate as f64;
    let out_len = ((input.len() as f64) / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = input.get(idx).copied().unwrap_or(0.0);
        let s1 = input.get(idx + 1).copied().unwrap_or(s0);
        out.push(s0 + (s1 - s0) * frac);
    }
    out
}

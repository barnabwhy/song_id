use std::{process::exit, sync::Arc, thread};

use tokio::{signal, sync::Mutex};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, StreamConfig,
};
use ringbuf::{
    storage::Heap,
    traits::{Consumer, Producer, Split},
    wrap::caching::Caching,
    HeapRb, SharedRb,
};

mod presence;
use presence::make_client;

mod shazam;
use shazam::core::http::try_recognize_song;
use shazam::fingerprinting::algorithm::SignatureGenerator;

use crate::presence::update_presence;

pub fn to_bytes(input: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 * input.len());

    for value in input {
        bytes.extend(&value.to_le_bytes());
    }

    bytes
}

#[tokio::main]
async fn main() {
    let host = cpal::default_host();

    let mut devices = host.input_devices().expect("no input device available");

    let device = devices
        .find(|x| x.name().unwrap().contains("USB AUDIO  CODEC"))
        .expect("no input device available");

    println!("Using device: {}", device.name().unwrap());

    let config = device
        .default_input_config()
        .expect("no default input config")
        .config();

    let seconds_per_read = 12;

    // Create a delay in case the input and output devices aren't synced.
    let latency_frames = seconds_per_read as f32 * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    // The buffer to share samples
    let ring = HeapRb::<i16>::new(latency_samples * 2);
    let (producer, mut consumer) = ring.split();

    let client = Arc::new(Mutex::new(
        make_client(discord_sdk::Subscriptions::ACTIVITY).await,
    ));
    let client2 = client.clone();

    tokio::spawn(async move {
        record_audio(producer, &device, &config).await.unwrap();
    });

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(seconds_per_read)).await;
            let popped: Vec<i16> = consumer.pop_iter().collect();
            if popped.len() == 0 {
                continue;
            }

            println!("Looking up with signature from {} samples ({}s)", popped.len(), popped.len() as f32 / 16_000.0);

            let fingerprint = SignatureGenerator::make_signature_from_buffer(&popped);
            let res = try_recognize_song(fingerprint).await;
            match res {
                Ok(song) => {
                    println!("Song recognized: {} - {}", song.song_name, song.artist_name);
                    update_presence(client2.lock().await, &song).await;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    client2.lock().await.discord.clear_activity().await.unwrap();
                }
            }
        }
    });

    println!("Recording audio in {}s intervals... Press Ctrl+C to stop.", seconds_per_read);

    signal::ctrl_c().await.expect("Failed to listen for event");

    println!("Received Ctrl+C event. Shutting down...");

    let client_lock = client.lock().await;

    match client_lock.discord.clear_activity().await {
        Ok(_) => {
            println!("Cleared discord activity");
        }
        Err(e) => {
            eprintln!("[DISCORD] Failed to clear discord activity: {}", e);
        }
    }

    exit(0);
}

async fn record_audio(
    mut producer: Caching<Arc<SharedRb<Heap<i16>>>, true, false>,
    device: &Device,
    config: &StreamConfig,
) -> anyhow::Result<()> {
    let sample_rate = config.sample_rate.0;
    let input_data_fn = move |data: &[i16], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        let data = samples_to_16khz(stereo_pcm_to_mono(data), sample_rate);
        if producer.push_slice(&data) == 0 {
            output_fell_behind = true;
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    };

    let err_fn = |err: cpal::StreamError| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let input_stream = device.build_input_stream(&config, input_data_fn, err_fn, None)?;

    input_stream.play()?;

    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
    // drop(input_stream);

    // Ok(())
}

fn stereo_pcm_to_mono(pcm: &[i16]) -> Vec<i16> {
    let mut mono = Vec::with_capacity(pcm.len() / 2);

    for i in (0..pcm.len()).step_by(2) {
        let sample = (pcm[i] as i32 + pcm[i + 1] as i32) / 2;
        mono.push(sample as i16);
    }

    mono
}

fn samples_to_16khz(samples: Vec<i16>, in_sample_rate: u32) -> Vec<i16> {
    if in_sample_rate == 16_000 {
        return samples;
    }

    if in_sample_rate % 16_000 != 0 {
        panic!("The input sample rate must be a multiple of 16_000");
    }

    let samples_to_merge = (in_sample_rate / 16_000) as usize;

    let mut res = Vec::with_capacity(samples.len() / samples_to_merge);
    for i in (0..samples.len()).step_by(samples_to_merge) {
        let mut sum: i32 = 0;
        for j in 0..samples_to_merge {
            sum += samples[i + j] as i32;
        }
        res.push((sum / samples_to_merge as i32) as i16)
    }
    res
}

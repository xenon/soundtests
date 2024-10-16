use std::fs::File;
use std::io::Write;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, Sample, SizedSample, StreamConfig, SupportedStreamConfig,
};
use wav::raw_audio_to_wav;

mod wav;

fn setup_default_device_default_config(quiet: bool) -> (Device, SupportedStreamConfig) {
    if !quiet {
        eprintln!("SETUP");
    }
    let host = cpal::default_host();
    let dev = host
        .default_output_device()
        .expect("Did not find output audio device!");
    if !quiet {
        eprintln!(
            "- Output device: {}",
            dev.name().expect("Device lacks a name..?")
        );
    }

    let conf = dev
        .default_output_config()
        .expect("Did not find default output config for device!");
    if !quiet {
        eprintln!("- Default output config: {:?}", conf);
    }

    let supported = dev
        .supported_output_configs()
        .expect("Could not list supported configs");
    if !quiet {
        eprintln!("- Supported output configs:");
        for (n, c) in supported.enumerate() {
            eprintln!("  {}. {:?}", n, c);
        }
    }
    (dev, conf)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WaveformKind {
    Silence,
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

#[derive(Clone, Debug)]
struct RunArgs {
    quiet: bool,
    waveforms: Vec<(WaveformKind, f32)>,
    generate_arrays: bool,
    cutoff: f32,
}

impl RunArgs {
    fn new(
        quiet: bool,
        waveforms: Vec<(WaveformKind, f32)>,
        generate_arrays: bool,
        cutoff: f32,
    ) -> Self {
        Self {
            quiet,
            waveforms,
            generate_arrays,
            cutoff,
        }
    }
}

impl Default for RunArgs {
    fn default() -> Self {
        Self {
            quiet: false,
            waveforms: vec![(WaveformKind::Silence, 0_f32)],
            generate_arrays: false,
            cutoff: 22050.0,
        }
    }
}

fn dispatch_run(dev: &Device, conf: SupportedStreamConfig, args: RunArgs) {
    use cpal::SampleFormat::*;
    match conf.sample_format() {
        I8 => run::<i8>(dev, conf.into(), args),
        I16 => run::<i16>(dev, conf.into(), args),
        I32 => run::<i32>(dev, conf.into(), args),
        I64 => run::<i64>(dev, conf.into(), args),
        U8 => run::<u8>(dev, conf.into(), args),
        U16 => run::<u16>(dev, conf.into(), args),
        U32 => run::<u32>(dev, conf.into(), args),
        U64 => run::<u64>(dev, conf.into(), args),
        F32 => run::<f32>(dev, conf.into(), args),
        F64 => run::<f64>(dev, conf.into(), args),
        f => panic!("Unknown sample format: {}", f),
    }
}

// Need these for calculating the period of a combination of waveforms
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b > 0 {
        let _b = b;
        b = a % b;
        a = _b;
    }
    a
}

fn lcm(a: usize, b: usize) -> usize {
    a * b / gcd(a, b)
}

const FAST_AMPLITUDE: bool = true;
fn calculate_amplitude(
    combined_period: usize,
    next_value: &Vec<fn(f32, f32, f32) -> f32>,
    sample_rate: f32,
    args: &RunArgs,
) -> f32 {
    if FAST_AMPLITUDE {
        // Makes some assumptions about max amplitude that I'm not sure I've proven to myself but seems to work
        // for the samples that we are using
        args.waveforms
            .iter()
            .filter_map(|(k, f)| {
                if k != &WaveformKind::Silence && f > &0.0_f32 {
                    Some((sample_rate / *f).ceil() as usize) // use ceil to ensure the combined waveform period is AT LEAST the entire length of the waveform, without ceil it could be to short from integer truncation
                } else {
                    None
                }
            })
            .count() as f32
    } else {
        // Samples a complete period and takes the max.. extremely naive
        let mut max = 0_f32;
        for sample_num in 0..combined_period {
            // Calculate current sample value,
            let mut acc = 0_f32;
            for (i, (_, f)) in args.waveforms.iter().enumerate() {
                acc += next_value[i](sample_num as f32, sample_rate, *f);
            }
            if acc > max {
                max = acc;
            }
        }
        max
    }
}

fn calculate_alpha(sample_rate: f32, cutoff: f32) -> f32 {
    let nc = cutoff / (sample_rate / 2.0);
    1.0 / (1.0 + std::f32::consts::PI / nc)
}

const CAP_ARRAY_GENERATION_SIZE: bool = true;
fn run<T: SizedSample + FromSample<f32>>(dev: &Device, conf: StreamConfig, args: RunArgs) {
    // Initialize constants
    let sample_rate = conf.sample_rate.0 as f32;
    let channels = conf.channels as usize;
    if !args.quiet {
        eprintln!("RUN");
        for sample in args.waveforms.iter() {
            eprintln!("- {:?} @ {}", sample.0, sample.1)
        }
        if args.waveforms.is_empty() {
            eprintln!("- You didn't add any samples to play...");
        }
    }

    // Initialize sample generator
    let mut next_value: Vec<fn(f32, f32, f32) -> f32> = Vec::with_capacity(args.waveforms.len());
    for sample in args.waveforms.iter() {
        next_value.push(match sample.0 {
            WaveformKind::Silence => |_, _, _| 0_f32,
            WaveformKind::Sine => |sample_clock, sample_rate, frequency| {
                ((2.0 * std::f32::consts::PI * frequency * sample_clock) / sample_rate).sin()
            },
            WaveformKind::Square => |sample_clock, sample_rate, frequency| {
                let period = sample_rate / frequency;
                if (sample_clock % period) < (period / 2.0) {
                    1.0
                } else {
                    -1.0
                }
            },
            WaveformKind::Sawtooth => |sample_clock, sample_rate, frequency| {
                let period = sample_rate / frequency;
                1_f32 - (2_f32 * (sample_clock % period) / period)
            },
            WaveformKind::Triangle => |sample_clock, sample_rate, frequency| {
                let period = sample_rate / frequency;
                let normalized_location = (sample_clock % period) / period;
                if normalized_location < 0.5 {
                    4_f32 * (normalized_location - 0.25_f32)
                } else {
                    1_f32 - 4_f32 * (normalized_location - 0.5_f32)
                }
            },
        });
    }

    // Find the max amplitude of the new waveform
    if !args.quiet {
        eprintln!("FIND AMPLITUDE:");
    }
    // 1 - Figure out the length of the new waveform:
    let mut combined_period = args
        .waveforms
        .iter()
        .filter_map(|(k, f)| {
            if k != &WaveformKind::Silence && f > &0.0_f32 {
                Some((sample_rate / *f).ceil() as usize) // use ceil to ensure the combined waveform period is AT LEAST the entire length of the waveform, without ceil it could be to short from integer truncation
            } else {
                None
            }
        })
        .reduce(lcm)
        .unwrap_or(1);
    if !args.quiet {
        eprintln!("- Period: {}", combined_period);
    }
    if combined_period as f32 >= sample_rate {
        if !args.quiet {
            eprintln!("- WARNING: Period is > 1 second worth of samples. Generation may be slow.");
        }
        if CAP_ARRAY_GENERATION_SIZE {
            if !args.quiet {
                eprintln!("- CAP ARRAY SIZE SET: Bounding the maximum sample count to 1 second worth of samples.");
            }
            combined_period = conf.sample_rate.0 as usize;
        }
    }

    // 2 - Calculate one period worth of samples and find the maximum amplitude
    let amplitude = calculate_amplitude(combined_period, &next_value, sample_rate, &args);
    // 3 - Debug print the max amplitude
    if !args.quiet {
        eprintln!("- Amplitude: {}", amplitude);
        if amplitude > 1.0 {
            eprintln!("- Amplitude is above threshold. Mix will be normalized.");
        } else {
            eprintln!("- Amplitude is below threshold. Mix does not need normalization.");
        }
    }
    let alpha = calculate_alpha(sample_rate, args.cutoff);
    // Generate one second worth of samples, write to a file then exit
    if args.generate_arrays {
        // Get the lowest freq (we are only taking up to that many samples for the array generation)
        // On failure we will only have 1 sample, you shouldn't pass this an empty vec...
        let mut vals: Vec<f32> = Vec::with_capacity(conf.sample_rate.0 as usize);
        let mut prev = 0.0;
        for sample_num in 0..combined_period {
            // Calculate current sample value
            let mut acc = 0_f32;
            for (i, (_, f)) in args.waveforms.iter().enumerate() {
                acc += next_value[i](sample_num as f32, sample_rate, *f);
            }
            // Normalize sample if necessary
            if amplitude > 1.0 {
                acc /= amplitude;
            }
            // Filter
            prev = alpha * acc + (1.0 - alpha) * prev;
            // Push
            vals.push(acc);
        }
        let mut file = File::create("samples.txt").expect("Failed to create file!");
        for val in vals.iter() {
            write!(file, "{} ", val).expect("Failed to write file!");
        }
        file.flush().expect("Failed to flush the file buffer");
        eprintln!("FILE WRITE SUCCESS...");

        let vals_u16: Vec<i16> = vals
            .iter()
            .map(|f| ((*f * 32768_f32).round() as i64).clamp(-32768, 32767) as i16)
            .collect();
        let bytes = raw_audio_to_wav(vals_u16, conf.sample_rate.0);
        let mut wavefile = File::create("samples.wav").expect("Failed to create file!");
        wavefile
            .write(&bytes)
            .expect("Failed to write the samples to wave!");
        eprintln!("WAVE FILE WRITE SUCCESS...");
        return;
    }

    let volume = 0.5;
    let mut sample_clock = 0f32;
    let mut prev = 0.0;
    let stream = dev
        .build_output_stream(
            &conf,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    // COPIED: Calculate current sample value
                    let mut acc = 0_f32;
                    // Sum the samples
                    for (i, (_, f)) in args.waveforms.iter().enumerate() {
                        acc += next_value[i](sample_clock, sample_rate, *f);
                    }
                    if amplitude > 1.0 {
                        acc /= amplitude;
                    }
                    // Filter
                    prev = alpha * acc + (1.0 - alpha) * prev;
                    let value: T = (prev * volume).to_sample::<T>();
                    sample_clock = (sample_clock + 1.0) % sample_rate;
                    for sample in frame.iter_mut() {
                        *sample = value;
                    }
                }
            },
            |err| eprintln!("An error occurred during output stream: {}", err),
            None,
        )
        .expect("Failed to create stream!");

    stream.play().expect("Failed to play the stream!");

    std::thread::sleep(std::time::Duration::from_millis(1000));
}

fn main() {
    let args = RunArgs::new(
        false,
        vec![
            //(WaveformKind::Square, 110.0),
            (WaveformKind::Sine, 440.0),
            //(WaveformKind::Triangle, 554.37),
            //(WaveformKind::Triangle, 659.25),
            (WaveformKind::Sine, 4400.0),
        ],
        false,
        441.0,
    );
    if !args.quiet {
        eprintln!("ARGUMENTS:");
        eprintln!("- {:?}", args);
    }
    let mut args2 = args.clone();
    args2.quiet = true;
    args2.generate_arrays = true;
    let (dev, conf) = setup_default_device_default_config(args.quiet);
    let conf2 = conf.clone();
    dispatch_run(&dev, conf, args);
    dispatch_run(&dev, conf2, args2);
}

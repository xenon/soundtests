use std::fs::File;
use std::io::Write;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, Sample, SizedSample, StreamConfig, SupportedStreamConfig,
};

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
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

#[derive(Clone, Debug)]
struct RunArgs {
    quiet: bool,
    frequency: f32,
    waveform: WaveformKind,
    generate_arrays: bool,
}

impl RunArgs {
    fn new(quiet: bool, frequency: f32, waveform: WaveformKind, generate_arrays: bool) -> Self {
        Self {
            quiet,
            frequency,
            waveform,
            generate_arrays,
        }
    }
}

impl Default for RunArgs {
    fn default() -> Self {
        Self {
            quiet: false,
            frequency: 440.0,
            waveform: WaveformKind::Sine,
            generate_arrays: false,
        }
    }
}

fn dispatch_run(dev: &Device, conf: SupportedStreamConfig, args: &RunArgs) {
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

fn run<T: SizedSample + FromSample<f32>>(dev: &Device, conf: StreamConfig, args: &RunArgs) {
    // Initialize constants
    let sample_rate = conf.sample_rate.0 as f32;
    let channels = conf.channels as usize;
    let frequency = args.frequency;
    if !args.quiet {
        eprintln!("RUN");
        eprintln!("- Frequency: {}Hz", frequency);
    }

    // Initialize sample generator
    let next_value: fn(f32, f32, f32) -> f32 = match args.waveform {
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
    };

    // Generate one second worth of samples, write to a file then exit
    if args.generate_arrays {
        let mut vals: Vec<f32> = Vec::with_capacity(conf.sample_rate.0 as usize);
        for sample_num in 0..(conf.sample_rate.0 as usize / frequency as usize + 1) {
            vals.push(next_value(sample_num as f32, sample_rate, frequency));
        }
        let mut file = File::create("samples.txt").expect("Failed to create file!");
        for val in vals {
            write!(file, "{} ", val).expect("Failed to write file!");
        }
        file.flush().expect("Failed to flush the file buffer");
        eprintln!("FILE WRITE SUCCESS... EXITING");
        return;
    }

    let volume = 0.5;
    let mut sample_clock = 0f32;
    let stream = dev
        .build_output_stream(
            &conf,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let value: T = (next_value(sample_clock, sample_rate, frequency) * volume)
                        .to_sample::<T>();
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
    let args = RunArgs::new(false, 440.0, WaveformKind::Triangle, false);
    if !args.quiet {
        eprintln!("ARGUMENTS:");
        eprintln!("- {:?}", args);
    }
    let (dev, conf) = setup_default_device_default_config(args.quiet);
    dispatch_run(&dev, conf, &args);
}

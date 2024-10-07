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

fn dispatch_run(dev: &Device, conf: SupportedStreamConfig, quiet: bool) {
    use cpal::SampleFormat::*;
    match conf.sample_format() {
        I8 => run::<i8>(dev, conf.into(), quiet),
        I16 => run::<i16>(dev, conf.into(), quiet),
        I32 => run::<i32>(dev, conf.into(), quiet),
        I64 => run::<i64>(dev, conf.into(), quiet),
        U8 => run::<u8>(dev, conf.into(), quiet),
        U16 => run::<u16>(dev, conf.into(), quiet),
        U32 => run::<u32>(dev, conf.into(), quiet),
        U64 => run::<u64>(dev, conf.into(), quiet),
        F32 => run::<f32>(dev, conf.into(), quiet),
        F64 => run::<f64>(dev, conf.into(), quiet),
        f => panic!("Unknown sample format: {}", f),
    }
}

fn run<T: SizedSample + FromSample<f32>>(dev: &Device, conf: StreamConfig, quiet: bool) {
    fn write_data<T: Sample + FromSample<f32>>(
        output: &mut [T],
        channels: usize,
        next_sample: &mut dyn FnMut() -> f32,
    ) {
        // multiply by volume=0.33... at the end to not play at full volume (save your ears!)
        let volume = 0.33333333;

        for frame in output.chunks_mut(channels) {
            let value: T = (next_sample() * volume).to_sample::<T>();
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }

    // Initialize constants
    let sample_rate = conf.sample_rate.0 as f32;
    let channels = conf.channels as usize;
    let frequency = 440.0;
    if !quiet {
        eprintln!("RUN");
        eprintln!("- Frequency: {}Hz", frequency);
    }

    // Initialize clock and sample value generator
    let mut sample_clock = 0f32;
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;
        ((2.0 * std::f32::consts::PI * frequency * sample_clock) / sample_rate).sin()
    };

    let stream = dev
        .build_output_stream(
            &conf,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                write_data(data, channels, &mut next_value)
            },
            |err| eprintln!("An error occurred during output stream: {}", err),
            None,
        )
        .expect("Failed to create stream!");

    stream.play().expect("Failed to play the stream!");

    std::thread::sleep(std::time::Duration::from_millis(1000));
}

fn main() {
    let quiet = false;
    let (dev, conf) = setup_default_device_default_config(quiet);
    dispatch_run(&dev, conf, quiet);
}

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::io::stdout;
use std::io::{stdin, Write};

use cpal::Stream;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, Sample, SizedSample, StreamConfig, SupportedStreamConfig,
};
use midir::{Ignore, MidiInput, MidiInputConnection};

fn setup_default_device_default_config(quiet: bool) -> (Device, SupportedStreamConfig) {
    if !quiet {
        eprintln!("SETUP OUTPUT:");
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

fn midi_to_frequency(note: u8) -> f32 {
    const A4: f32 = 440.0;
    A4 * 2f32.powf((note as f32 - 69.0) / 12.0)
}

fn midi_velocity_to_loudness(velocity: u8) -> f32 {
    const GUESS_EXP_FOR_PERCEIVED_LOUDNESS: f32 = 2.0; // TODO: this is not scientific and is an estimate
    (velocity as f32 / 127.0).powf(GUESS_EXP_FOR_PERCEIVED_LOUDNESS)
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum MidiEvent {
    KeyOff(u8),
    KeyOn(u8, u8),
}

fn setup_midi_device(quiet: bool) -> (Receiver<MidiEvent>, MidiInputConnection<()>) {
    if !quiet {
        eprintln!("SETUP MIDI:")
    }

    let mut midi_in = MidiInput::new("midir reading input").expect("could not read midi input!");
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => panic!("no input port found"),
        1 => {
            println!(
                "- Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\n- Available input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("- {}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("- Please select input port: ");
            stdout().flush().expect("could not flush stdout");
            let mut input = String::new();
            stdin().read_line(&mut input).expect("could not read a line");
            in_ports
                .get(input.trim().parse::<usize>().expect("could not parse port number"))
                .expect("invalid input port selected")
        }
    };

    if !quiet {
        eprintln!("\n- Opening connection");
    }
    let in_port_name = midi_in.port_name(in_port).expect("could not open midi port!");

    let (send, recv) = mpsc::channel();
    
    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message, _| {
            //println!("{}: {:?} (len = {})", stamp, message, message.len());
            if message.len() == 3 {
                if (message[0] & 0x0F) != 0 {
                    if !quiet {
                        eprintln!("Only support midi channel 0, received: {}", (message[0] & 0x0F));
                    }
                }
                match message[0] & 0xF0 {
                    0b10000000 => {
                        //eprintln!("Note off: {} @ velocity {}", message[1], message[2]);
                        send.send(MidiEvent::KeyOff(message[1])).expect("channel closed!");
                    },
                    0b10010000 => {
                        //eprintln!("Note on: {} @ velocity {}", message[1], message[2]);
                        send.send(MidiEvent::KeyOn(message[1], message[2])).expect("channel closed!");
                    },
                    x => {
                        if !quiet {
                            eprintln!("Unknown message type: {}", x);
                        }
                    }
                }

            } else {
                if !quiet {
                    eprintln!("* Unknown midi message pack. Length not 3! {:?}", message);
                }
            }
        },
        (),
    ).expect("could not create connection!");

    if !quiet {
        eprintln!(
            "- Connection open, reading input from '{}' (press ctrl+c to exit) ...",
            in_port_name
        );
    }
    (recv, _conn_in)
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
    waveform: WaveformKind,
}

impl RunArgs {
    fn new(quiet: bool, waveform: WaveformKind) -> Self {
        Self {
            quiet,
            waveform,
        }
    }
}

impl Default for RunArgs {
    fn default() -> Self {
        Self {
            quiet: false,
            waveform: WaveformKind::Sine,
        }
    }
}

fn dispatch_run(dev: &Device, conf: SupportedStreamConfig, args: RunArgs, recv: Receiver<MidiEvent>) -> Stream  {
    use cpal::SampleFormat::*;
    match conf.sample_format() {
        I8 => run::<i8>(dev, conf.into(), args, recv),
        I16 => run::<i16>(dev, conf.into(), args, recv),
        I32 => run::<i32>(dev, conf.into(), args, recv),
        I64 => run::<i64>(dev, conf.into(), args, recv),
        U8 => run::<u8>(dev, conf.into(), args, recv),
        U16 => run::<u16>(dev, conf.into(), args, recv),
        U32 => run::<u32>(dev, conf.into(), args, recv),
        U64 => run::<u64>(dev, conf.into(), args, recv),
        F32 => run::<f32>(dev, conf.into(), args, recv),
        F64 => run::<f64>(dev, conf.into(), args, recv),
        f => panic!("Unknown sample format: {}", f),
    }
}

fn run<T: SizedSample + FromSample<f32>>(dev: &Device, conf: StreamConfig, args: RunArgs, recv: Receiver<MidiEvent>) -> Stream {
    // Initialize constants
    let sample_rate = conf.sample_rate.0 as f32;
    let channels = conf.channels as usize;
    if !args.quiet {
        eprintln!("RUN");
        eprintln!("- Sound: {:?}", args.waveform);
    }

    // Initialize sample generator
    let next_sample_fn: fn(f32, f32, f32) -> f32 = match args.waveform {
            WaveformKind::Sine => |sample_clock, sample_rate, frequency| {
                let period = sample_rate / frequency;
                let normalized_location = (sample_clock % period) / period;
                (2.0 * std::f32::consts::PI * normalized_location).sin()
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

    let volume = 0.3333;
    let mut sample_clock = 0f32;
    let mut playing = HashMap::new();
    let mut amplitude = 0.0_f32;
    let stream = dev
        .build_output_stream(
            &conf,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    // Check if playing sounds changed
                    let mut changed = false;
                    while let Ok(event) = recv.try_recv() {
                        match event {
                            MidiEvent::KeyOff(note) => {
                                playing.remove(&note);
                            },
                            MidiEvent::KeyOn(note, velocity) => {
                                playing.insert(note,  midi_velocity_to_loudness(velocity));
                            },
                        }
                        changed = true;
                    }
                    // Update amplitude of changed signal, hack clock to 0 if all notes released
                    if changed {
                        if playing.is_empty() {
                            sample_clock = 0f32;
                        }
                        amplitude = 0_f32;
                        for (_, v) in playing.iter() {
                            amplitude += v;
                        }
                    }
                    // MIX:
                    let mut acc = 0_f32;
                    if playing.len() > 0 {
                        // Sum the samples
                        for (n, v) in playing.iter() {
                            acc += v * next_sample_fn(sample_clock, sample_rate, midi_to_frequency(*n));
                        }
                        if amplitude > 1.0 {
                            acc /= amplitude;
                        }
                    }
                    let value: T = (acc * volume).to_sample::<T>();
                    sample_clock = sample_clock + 1.0;
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
    stream
}

fn main() {
    let args = RunArgs::new(
        false,
        WaveformKind::Sine,
    );
    if !args.quiet {
        eprintln!("ARGUMENTS:");
        eprintln!("- {:?}", args);
    }
    let (dev, conf) = setup_default_device_default_config(args.quiet);
    let (recv, _midi_handle) = setup_midi_device(args.quiet);
    let _stream = dispatch_run(&dev, conf, args, recv);

    // wait for ctrl c example code
    let (tx, rx) = mpsc::channel();
    
    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    rx.recv().expect("Could not receive from channel.");
}

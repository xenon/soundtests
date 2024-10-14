const WAV_HEADER_SIZE: usize = 44;
const WAV_CHANNELS: u16 = 1;
const WAV_BPS: u16 = 16;

fn make_wav_header(o: &mut Vec<u8>, sample_rate: usize, sample_count: usize) {
    let str_bytes: fn(&str) -> Vec<u8> = |v| v.chars().map(|c| c as u8).collect();
    let u32_bytes: fn(usize) -> Vec<u8> = |u| (u as u32).to_le_bytes().to_vec();
    let u16_bytes: fn(u16) -> Vec<u8> = |u| u.to_le_bytes().to_vec();

    o.append(&mut str_bytes("RIFF"));
    o.append(&mut u32_bytes(WAV_HEADER_SIZE));

    o.append(&mut str_bytes("WAVE"));

    o.append(&mut str_bytes("fmt "));
    o.append(&mut u32_bytes(16)); // header size
    o.append(&mut u16_bytes(1)); // tag
    o.append(&mut u16_bytes(WAV_CHANNELS)); // channels
    o.append(&mut u32_bytes(sample_rate)); // sample rate
    o.append(&mut u32_bytes(sample_rate * (WAV_BPS as usize / 8))); // data rate
    o.append(&mut u16_bytes(1)); // block size
    o.append(&mut u16_bytes(WAV_BPS)); // bits per sample

    o.append(&mut str_bytes("data"));
    o.append(&mut u32_bytes(sample_count * (WAV_BPS as usize / 8)));
}

pub fn raw_audio_to_wav(samples: Vec<i16>, sample_rate: u32) -> Vec<u8> {
    let parity = samples.len() % 2;

    eprintln!("- Calculated length: {}", samples.len());

    let mut o = Vec::with_capacity(samples.len() + WAV_HEADER_SIZE);
    make_wav_header(&mut o, sample_rate as usize, samples.len());
    o.append(
        &mut samples
            .into_iter()
            .map(|s| s.to_le_bytes())
            .flatten()
            .collect::<Vec<u8>>(),
    );

    // wav files use 16bit ints, so we need an even number of bytes otherwise it's technically invalid
    if parity == 1 {
        o.push(0);
    }

    o
}

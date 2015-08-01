extern crate alsa;

use std::io::Write;
use std::ffi::CString;
use alsa::Direction;
use alsa::pcm::{PCM, HwParams, Format, Access, State};

const SAMPLE_RATE: u32 = 44100;
// Smallest phase - phase of highest frequency
// Choose 55Hz - 110Hz for now, so:
const PHASE_MIN: usize = 401; // 44100/110
const PHASE_MAX: usize = 802; // 44100/55

const SAMPLES: usize = PHASE_MAX*2;

type Sample = i16;

struct Config {
    base_frequency: f64,
    card: CString,
    notes: Box<Vec<String>>,
    sample_rate: u32
}

fn default_config() -> Config {
    Config {
        base_frequency: 440.0,
        card: CString::new("default").unwrap(),
        // guitar standard
        notes: Box::new(vec!(
            "E2".to_string(),
            "A2".to_string(),
            "D3".to_string(),
            "G3".to_string(),
            "B3".to_string(),
            "E4".to_string())),
        sample_rate: 44100
    }
}

type Phase = usize;

fn phase(config: &Config, pitch: Pitch) -> Phase {
    (config.sample_rate as f64 / config.base_frequency / (2.0_f64).powf(1.0 / 12.0).powi(pitch as i32)).round() as Phase
}

#[test]
fn phase_default_a4() {
    assert_eq!(100, phase(&default_config(), 0));
}

/// Pitch is the number of half steps up from A4.
type Pitch = isize;

fn parse_note(c: char) -> Option<isize> {
    match c {
        'A' => Some(0),
        'B' => Some(2),
        'C' => Some(3),
        'D' => Some(5),
        'E' => Some(7),
        'F' => Some(8),
        'G' => Some(10),
        _   => None
    }
}

fn parse_octave(c: char) -> Option<isize> {
    match c {
        '0' => Some(-4),
        '1' => Some(-3),
        '2' => Some(-2),
        '3' => Some(-1),
        '4' => Some(0),
        '5' => Some(1),
        '6' => Some(2),
        '7' => Some(3),
        '8' => Some(4),
        _   => None
    }
}

fn parse_alteration(c: char) -> Option<isize> {
    match c {
        // TODO there are multiple unicode symbols for each of these... :/
        '♯' => Some(1),
        '#' => Some(1),
        '♮' => Some(0),
        'b' => Some(-1),
        '♭' => Some(-1),
        _ => None
    }
}

fn parse_pitch(string: &String) -> Pitch {
    let note = parse_note(string.chars().nth(0).unwrap()).unwrap();
    match string.chars().nth(1) {
        None => note,
        Some(c) => match (parse_alteration(c), parse_octave(c)) {
            (Some(alt), None) => match string.chars().nth(2) {
                None => note + alt,
                Some(c) => note + parse_octave(c).unwrap()*12 + alt
            },
            (None, Some(oct)) => note + oct*12,
            _ => unreachable!()
        }
    }
}

fn pprint_pitch(pitch: Pitch) -> String {
    let pitch = (48 + pitch) as usize;
    let notes = vec!["A", "A♯", "B", "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯"];
    let octaves = vec!["0", "1", "2", "3", "", "5", "6", "7", "8"];
    notes[pitch % 12].to_string() + octaves[pitch / 12]
}

#[test]
fn parse_and_pprint_pitch() {
    for pitch in -48..59 {
        assert_eq!(pitch, parse_pitch(&pprint_pitch(pitch)));
    }
}

// Phase (as float, if we ever average over phases) to
// pitch (difference from A4, as float). Round to get
// the closest note. Note that the sign is important when
// figuring out whether you're sharp or flat.
fn frequency(config: &Config, phase: f64) -> f64 {
    let f = config.base_frequency;
    let s = config.sample_rate as f64;
    let p = phase;
    (s / (p * f)).log(2_f64.powf(1.0 / 12.0))
}

fn calculate_phase_boundaries(config: &Config) -> Vec<Phase> {
    let mut notes: Vec<Pitch> = config.notes.iter().map(parse_pitch).collect();
    notes.sort();
    let b_len = notes.len() * 2 + 1;
    let mut boundaries: Vec<Phase> = Vec::with_capacity(b_len);
    // Should probably use Vec::from_elem(b_len, 0) but that is not in stable yet
    unsafe { boundaries.set_len(b_len); }
    boundaries[0] = phase(config, notes[0] - 3);
    boundaries[b_len-1] = phase(config, notes[notes.len()-1] + 3);
    for (note_index, note) in notes.iter().enumerate() {
        boundaries[note_index*2 + 1] = phase(config, *note);
    }
    for note_index in 0..notes.len()-1 {
        boundaries[note_index*2 + 2] =
            (boundaries[note_index*2 + 1] + boundaries[note_index*2 + 3]) / 2;
    }
    boundaries
}

fn error_squared(a: Sample, b: Sample) -> u64 {
    let d = (a as i32 - b as i32) as i64;
    (d*d) as u64
}

fn window_error(data: &[Sample], offset: usize, error_limit: u64) -> u64 {
    let mut error = 0;
    for i in 0..PHASE_MAX {
        error += error_squared(data[i], data[i+offset]);
        if error >= error_limit {
            break;
        }
    }
    error
}

fn autocorrelate(data: &[Sample]) -> Phase {
    let mut min_error = window_error(data, PHASE_MIN, u64::max_value());
    let mut min_phase = PHASE_MIN;
    for phase in PHASE_MIN + 1 .. PHASE_MAX {
        let error = window_error(data, phase, min_error);
        if error < min_error {
            min_error = error;
            min_phase = phase;
        }
    }
    min_phase
}

// Frequency range:
// Cello & Guitar open strings
// + some room downwards for drop tunings (1 tone?)
// + some room both directions for being off (semitone?)

// Actually, autocorrelation is terrible with octaves anyways, so why not
// just go over one octave? Preferably the lowest, I think.

fn main() {
    // make_noise();
    let config = default_config();

    {
        let pcm = PCM::open(&*config.card, Direction::Capture, false).unwrap();

        let hwp = HwParams::any(&pcm).unwrap();
        hwp.set_channels(1).unwrap();
        hwp.set_rate(SAMPLE_RATE, 0).unwrap();
        hwp.set_format(Format::s16()).unwrap();
        hwp.set_access(Access::RWInterleaved).unwrap();
        pcm.hw_params(&hwp).unwrap();
        let io = pcm.io_i16().unwrap();

        pcm.prepare().unwrap();
        loop {
            let mut data = [0i16; SAMPLES];
            assert_eq!(io.readi(&mut data).unwrap(), SAMPLES);
            let phase = autocorrelate(&data);
            // VT100 escape magic to clear the current line and reset the cursor
            print!("\x1B[2K\r");
            print!("phase: {}, freq: {:.3}, pitch: {:.3}, note: {}", phase, SAMPLE_RATE as f64 / phase as f64, frequency(&config, phase as f64), pprint_pitch(frequency(&config, phase as f64).round() as isize));
            std::io::stdout().flush().unwrap();
        }
    }
    const n :usize = 100;
    let mut buf = [0i16; n*1024];

    {
        let pcm = PCM::open(&*config.card, Direction::Capture, false).unwrap();

        let hwp = HwParams::any(&pcm).unwrap();
        hwp.set_channels(1).unwrap();
        hwp.set_rate(SAMPLE_RATE, 0).unwrap();
        hwp.set_format(Format::s16()).unwrap();
        hwp.set_access(Access::RWInterleaved).unwrap();
        pcm.hw_params(&hwp).unwrap();
        let io = pcm.io_i16().unwrap();

        pcm.prepare().unwrap();
        assert_eq!(io.readi(&mut buf[..]).unwrap(), n*1024); //2*44100);//, 1024);

        pcm.drain().unwrap();
    }

    println!("playback!");

    let pcm = PCM::open(&*config.card, Direction::Playback, false).unwrap();

    // Set hardware parameters: 44100 Hz / Mono / 16 bit
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(SAMPLE_RATE, 0).unwrap();
    hwp.set_format(Format::s16()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    let io = pcm.io_i16().unwrap();

    // Play recording back for 2 seconds.
    //for _ in 0..2*44100/1024 {
        assert_eq!(io.writei(&buf[..]).unwrap(), n*1024);//2*44100);//, 1024);
    //}

    // In case the buffer was larger than 2 seconds, start the stream manually.
    if pcm.state() != State::Running { pcm.start().unwrap() };
    // Wait for the stream to finish playback.
    pcm.drain().unwrap();
}

fn make_noise() {
    // Open default playback device
    let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Playback, false).unwrap();

    // Set hardware parameters: 44100 Hz / Mono / 16 bit
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(44100, 0).unwrap();
    hwp.set_format(Format::s16()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    let io = pcm.io_i16().unwrap();

    // Make a sine wave
    let mut buf = [0i16; 1024];
    for (i, a) in buf.iter_mut().enumerate() {
        *a = ((i as f32 * 2.0 * ::std::f32::consts::PI / 128.0).sin() * 8192.0) as i16
    }

    // Play it back for 2 seconds.
    for _ in 0..2*44100/1024 {
        assert_eq!(io.writei(&buf[..]).unwrap(), 1024);
    }

    // In case the buffer was larger than 2 seconds, start the stream manually.
    if pcm.state() != State::Running { pcm.start().unwrap() };
    // Wait for the stream to finish playback.
    pcm.drain().unwrap();
}

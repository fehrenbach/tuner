extern crate alsa;

use std::ops::Sub;
use std::io::Write;
use std::ffi::CString;
use alsa::Direction;
use alsa::pcm::{PCM, HwParams, Format, Access};

type Sample = i16;

/// A Pitch is the number of half steps up from A4.
type Pitch = isize;

struct Config {
    base_frequency: f64,
    card: CString,
    pitches: Box<[Pitch]>,
    sample_rate: u32
}

fn parse_pitches(s: &str) -> Box<[Pitch]> {
    let mut res: Vec<Pitch> = s.split(" ").map(parse_pitch).collect();
    res.sort();
    res.into_boxed_slice()
}

#[test]
fn parse_pitches_guitar() {
    let computed = parse_pitches("E2 A2 D3 G3 B3 E4");
    let expected = [-29, -24, -19, -14, -10, -5];
    for i in 0..computed.len() {
        assert_eq!(expected[i], computed[i]);
    }
}

fn default_config() -> Config {
    Config {
        base_frequency: 440.0,
        card: CString::new("default").unwrap(),
        // guitar standard
        pitches: parse_pitches("E2 A2 D3 G3 B3 E4"),
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

fn parse_note(c: char) -> Option<isize> {
    match c {
        'C' => Some(-9),
        'D' => Some(-7),
        'E' => Some(-5),
        'F' => Some(-4),
        'G' => Some(-2),
        'A' => Some(0),
        'B' => Some(2),
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

fn parse_pitch(string: &str) -> Pitch {
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

#[test]
fn parse_pitch_tests() {
    assert_eq!(0, parse_pitch("A"));
    assert_eq!(0, parse_pitch("A4"));
    assert_eq!(1, parse_pitch("A#"));
    assert_eq!(-1, parse_pitch("Ab"));
    assert_eq!(12, parse_pitch("A5"));
    assert_eq!(-12, parse_pitch("A3"));
    assert_eq!(-5, parse_pitch("E4"));
    assert_eq!(7, parse_pitch("E5"));
    assert_eq!(-57, parse_pitch("C0"));
    assert_eq!(-45, parse_pitch("C1"));
    assert_eq!(-33, parse_pitch("C2"));
    assert_eq!(-21, parse_pitch("C3"));
    assert_eq!(-9, parse_pitch("C4"));
    assert_eq!(3, parse_pitch("C5"));
    assert_eq!(15, parse_pitch("C6"));
    assert_eq!(27, parse_pitch("C7"));
    assert_eq!(39, parse_pitch("C8"));
}

fn pprint_pitch(pitch: Pitch) -> String {
    let pitch = (48 + pitch + 9) as usize;
    let notes = vec!["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
    let octaves = vec!["0", "1", "2", "3", "", "5", "6", "7", "8"];
    notes[pitch % 12].to_string() + octaves[pitch / 12]
}

#[test]
fn pprint_pitch_tests() {
    assert_eq!("A", pprint_pitch(0));
    assert_eq!("A♯", pprint_pitch(1));
    assert_eq!("G♯", pprint_pitch(-1));
    assert_eq!("C0", pprint_pitch(-57));
    assert_eq!("C", pprint_pitch(-9));
    assert_eq!("C5", pprint_pitch(3));
    assert_eq!("C8", pprint_pitch(39));
}

#[test]
fn parse_and_pprint_pitch() {
    for pitch in -57..39 {
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

/*
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
*/

fn error_squared(a: Sample, b: Sample) -> u64 {
    let d = (a as i32 - b as i32) as i64;
    (d*d) as u64
}

fn window_error(data: &[Sample], offset: usize, error_limit: u64, phase_max: Phase) -> u64 {
    let mut error = 0;
    for i in 0..phase_max {
        error += error_squared(data[i], data[i+offset]);
        if error >= error_limit {
            break;
        }
    }
    error
}

fn autocorrelate(phase_min: Phase, phase_max: Phase, data: &[Sample]) -> Phase {
    let mut min_error = window_error(data, phase_min, u64::max_value(), phase_max);
    let mut min_phase = phase_min;
    for phase in phase_min+1 .. phase_max {
        let error = window_error(data, phase, min_error, phase_max);
        if error < min_error {
            min_error = error;
            min_phase = phase;
        }
    }
    min_phase
}

fn difference<T>(a: T, b: T) -> T
    where T : Ord + Sub<Output=T> {
    if a > b { a - b } else { b - a }
}

// TODO add a couple of these examples-inside-documentation tests
// Corner cases: empty slice, larger/smaller than slice extremes, duplicates, ...
/// Takes a sorted slice and a value of element type and returns the index of
/// the element closest in value.
fn closest<T>(x: T, xs: &[T]) -> usize
    where T : Ord + Sub<Output=T> + Copy {
    match xs.binary_search(&x) {
        Ok(i) => i,
        Err(0) => 0,
        Err(i) =>
            if difference(xs[i], x) < difference(xs[i-1], x) { i } else { i-1 }
    }
}

fn main() {
    let config = default_config();

    let mut phases: Vec<Phase> = (*config.pitches).iter().map(|&p| phase(&config, p)).collect();
    phases.sort();
    let phases = phases;

    let phase_min = phases[0];
    let phase_max = phases[phases.len()-1];
    let sample_rate = config.sample_rate;
    let samples = phase_max * 2;

    let mut backing_vector: Vec<Sample> = Vec::with_capacity(samples);
    // Should probably use Vec::from_elem(samples, 0) but that is not in stable yet
    unsafe { backing_vector.set_len(samples); }
    let mut data = &mut backing_vector[..];

    let pcm = PCM::open(&*config.card, Direction::Capture, false).unwrap();
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(sample_rate, 0).unwrap();
    hwp.set_format(Format::s16()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    let io = pcm.io_i16().unwrap();
    pcm.prepare().unwrap();

    loop {
        assert_eq!(io.readi(&mut data).unwrap(), samples);
        let phase = autocorrelate(phase_min, phase_max, &data);
        let closest_index = closest(phase, &phases);
        // VT100 escape magic to clear the current line and reset the cursor
        print!("\x1B[2K\r");
        print!("phase:{:>4}, freq:{:>8.3}, pitch:{:>8.3}, note: {}, string: {}", phase, sample_rate as f64 / phase as f64, frequency(&config, phase as f64), pprint_pitch(frequency(&config, phase as f64).round() as isize), closest_index + 1);
        std::io::stdout().flush().unwrap();
    }
}

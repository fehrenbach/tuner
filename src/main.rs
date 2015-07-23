extern crate alsa;

use std::ffi::CString;
use alsa::Direction;
use alsa::pcm::{PCM, HwParams, Format, Access, State};

fn main() {
    // make_noise();

    const n :usize = 100;
    let mut buf = [0i16; n*1024];

    {
        let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Capture, false).unwrap();

        let hwp = HwParams::any(&pcm).unwrap();
        hwp.set_channels(1).unwrap();
        hwp.set_rate(44100, 0).unwrap();
        hwp.set_format(Format::s16()).unwrap();
        hwp.set_access(Access::RWInterleaved).unwrap();
        pcm.hw_params(&hwp).unwrap();
        let io = pcm.io_i16().unwrap();

        pcm.prepare().unwrap();
        assert_eq!(io.readi(&mut buf[..]).unwrap(), n*1024); //2*44100);//, 1024);

        pcm.drain().unwrap();
    }

    println!("playback!");

    let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Playback, false).unwrap();

    // Set hardware parameters: 44100 Hz / Mono / 16 bit
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(44100, 0).unwrap();
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

use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub(crate) fn run_single_mode(path: &impl AsRef<Path>, should_loop: bool) {
    let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
        eprintln!("Failed to open the default audio device");
        return;
    };

    let sink = match Sink::try_new(&stream_handle) {
        Ok(sink) => sink,
        Err(e) => {
            eprintln!("Play error: {}", e);
            return;
        }
    };

    let file = BufReader::new(match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Failed to open file");
            return;
        }
    });

    let Ok(source) = Decoder::new(file) else {
        eprintln!("Failed to decode the file");
        return;
    };

    if should_loop {
        sink.append(source.repeat_infinite());
    } else {
        sink.append(source);
    }

    sink.sleep_until_end();
}

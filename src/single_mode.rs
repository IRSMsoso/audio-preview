use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub(crate) fn run_single_mode(path: &impl AsRef<Path>, should_loop: bool) {
    let Ok(output_stream) = OutputStreamBuilder::open_default_stream() else {
        eprintln!("Failed to open the default audio device");
        return;
    };

    let sink = Sink::connect_new(output_stream.mixer());

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

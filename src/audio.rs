use rodio::{
    OutputStream, OutputStreamBuilder, Sink,
    source::{SineWave, Source},
};
use std::time::Duration;

pub struct AudioHandler {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioHandler {
    pub fn new() -> Option<Self> {
        let _stream =
            OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        let sink = Sink::connect_new(&_stream.mixer());

        let source = SineWave::new(440.0)
            .take_duration(Duration::from_secs_f32(10.0))
            .amplify(0.20);

        sink.append(source.repeat_infinite());
        sink.pause();

        Some(Self { _stream, sink })
    }

    pub fn play(&self) {
        if self.sink.is_paused() {
            self.sink.play();
        }
    }

    pub fn pause(&self) {
        if !self.sink.is_paused() {
            self.sink.pause();
        }
    }
}

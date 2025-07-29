use std::{
    fs, path::PathBuf, sync::{atomic::{AtomicU8, Ordering}, Arc, Mutex, Once}
};

use num_enum::TryFromPrimitive;
use bytes::Bytes;
use anyhow::Error;
use crossbeam::channel;
use directories::ProjectDirs;
use nih_plug::prelude::*;

use crate::libplugui::{IcedState};

mod libplugui;
mod editor;

static INIT_FILES: Once = Once::new();
type SysEx = ();

fn init_metadata() {
    INIT_FILES.call_once( || {
        // TODO: find production-ready way to get logger to output to a specific file (might need to
        // fork nih_plug)

        let _ = nih_plug::wrapper::setup_logger();
        nih_log!("Logger initialized");
        nih_log!("data directory path: {}", env!("NIH_LOG"));
        std::panic::set_hook(
            Box::new(|info| {
                nih_error!("PANIC: {}", info);
            })
        )
    });
}

fn init_data_dir() -> PathBuf {
    let proj = ProjectDirs::from("com", "Andrew Marous", "Ahmad")
        .expect("No valid home directory for plugin.");
    let dir = proj.data_dir();
    fs::create_dir_all(dir).expect("A directory in ProjectDirs::from().data_dir() doesn't exist.");
    dir.to_path_buf()
}

pub struct Ahmad {
    params: Arc<AhmadParams>,
    agent_stream: Arc<channel::Receiver<Result<Bytes, Error>>>,

    // audio output fields
    is_playing: bool,
}

#[derive(Params)]
pub struct AhmadParams {
    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,

    #[persist = "filetype"]
    filetype: Arc<AtomicU8>,
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum ResponseFiletype {
    Wav = 0,
    Midi = 1,
}

#[derive(Debug)]
pub enum BufferType {
    Wav(Vec<f32>),
    Midi(Vec<NoteEvent<SysEx>>),
    Empty,
}

impl Default for Ahmad {
    fn default() -> Self {
        let (_, stream) = channel::unbounded::<Result<Bytes, Error>>();
        Self {
            params: Arc::new(AhmadParams::default()),
            agent_stream: Arc::new(stream),
            is_playing: false,
        }
    }
}

impl Default for AhmadParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            filetype: Arc::new(AtomicU8::new(ResponseFiletype::Wav as u8)),
        }
    }
}

impl Ahmad {
    fn process_wav_chunk(&mut self, bytes: &Bytes) -> Vec<f32> {
            bytes
            .chunks_exact(2)
            .map(|chunk| {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                sample as f32 / 32768.0 // TODO: confirm this
            })
            .collect()
    }

    fn process_midi_chunk(&mut self, bytes: &Bytes) -> Vec<nih_plug::midi::NoteEvent<SysEx>> {
        // TODO: find midi library to process events in a stream
        // if none exist, do these things in order:
        //  1. Block stream until entire file is received, then construct file and read as normal
        //  2. Write library to process MIDI file in chunks

        Vec::new()
    }

    /// returns a Vec<f32> containing the currently-available chunks of `self.agent_stream`.
    /// This is meant to be wrapped in a BufferType with BufferType::from() if generic handling is
    /// needed.
    fn get_wav_buffer(&mut self) -> (Vec<f32>, usize) {
        assert!(matches!(
            ResponseFiletype::try_from_primitive(
                self.params.filetype.load(Ordering::Relaxed)
            ).expect("Response filetype not valid."),
            ResponseFiletype::Wav
        ));
        let mut buf = vec![0.0; 2048];
        let mut buf_pos = 0usize;
        while let Ok(item) = self.agent_stream.try_recv() {
            match item {
                Ok(chunk) => {
                    let samples = self.process_wav_chunk(&chunk);
                    for sample in samples {
                        buf[buf_pos] = sample;
                        buf_pos = (buf_pos + 1) % buf.len()
                    }
                }
                Err(e) => {
                    nih_error!("Error receiving response bytes on plugin layer: {}", e);
                    return (Vec::new(), 0usize);
                }
            }
        }
        (buf, buf_pos)
    }

    fn get_midi_buffer(&mut self) -> Vec<NoteEvent<SysEx>> {
        assert!(matches!(
            ResponseFiletype::try_from_primitive(
                self.params.filetype.load(Ordering::Relaxed)
            ).expect("Response filetype not valid."),
            ResponseFiletype::Wav
        ));

        let buf: Vec<NoteEvent<SysEx>> = Vec::new();

        // TODO: process MIDI stream

        buf
    }
}


impl Plugin for Ahmad {
    const NAME: &'static str = "ahmad";
    const VENDOR: &'static str = "Andrew Marous";
    const URL: &'static str = "https://github.com/andrewmarous";
    const EMAIL: &'static str = "andrewmarous@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = SysEx;
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // TODO: create channel, pass sender down to agent

        let (tx, rx) = channel::unbounded::<Result<Bytes, Error>>();
        self.agent_stream = Arc::new(rx);
        editor::create(
            self.params.clone(),
            Arc::new(tx)
        )
    }

    fn initialize(
            &mut self,
            _audio_io_layout: &AudioIOLayout,
            _buffer_config: &BufferConfig,
            _context: &mut impl InitContext<Self>,
        ) -> bool {
        init_metadata();

        true
    }

    fn process(
            &mut self,
            buffer: &mut Buffer,
            _aux: &mut AuxiliaryBuffers,
            _context: &mut impl ProcessContext<Self>,
        ) -> ProcessStatus {

        let filetype = ResponseFiletype::try_from_primitive(
            self.params.filetype.load(Ordering::Relaxed)
        )
            .expect("Response filetype is not valid.");

        match filetype {
            ResponseFiletype::Wav => {
                let (gen_buffer, mut gen_buffer_pos) = self.get_wav_buffer();
                for mut channel_samples in buffer.iter_samples() {
                    for (channel, sample) in channel_samples.iter_mut().enumerate() {
                        // TODO: ensure this is right
                        let gen_sample = if gen_buffer_pos < gen_buffer.len() {
                            gen_buffer[gen_buffer_pos]
                        } else { 0.0 };

                        *sample = gen_sample;

                        if channel == 0 {
                            gen_buffer_pos = (gen_buffer_pos + 1) % gen_buffer.len();
                        }
                    }
                }
            },
            ResponseFiletype::Midi => {
                let gen_buffer = self.get_midi_buffer();

                panic!("MIDI files not implemented yet.")

                // TODO:
            }
        }
        // add audio to real buffer

        for mut channel_samples in buffer.iter_samples() {
            if self.params.editor_state.is_open() {
                // do some processing only when window is open
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Ahmad {
    const VST3_CLASS_ID: [u8; 16] = *b"ahmadfoobarfooba";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

// add logging and steinberg API safety

nih_export_vst3!(Ahmad);


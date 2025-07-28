use std::{
    fs, path::PathBuf, sync::{atomic::AtomicU8, Arc, Once}
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

    agent_stream: Arc<channel::Receiver<Result<Bytes, Error>>>
}

#[derive(Params)]
pub struct AhmadParams {
    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,

    #[persist = "filetype"]
    filetype: Arc<AtomicU8>
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum ResponseFiletype {
    Wav = 0,
    Midi = 1,
}

impl Default for Ahmad {
    fn default() -> Self {
        let (_, stream) = channel::unbounded::<Result<Bytes, Error>>();
        Self {
            params: Arc::new(AhmadParams::default()),
            agent_stream: Arc::new(stream)
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

    type SysExMessage = ();
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
        for channel_samples in buffer.iter_samples() {
            // do some audio processing

            if self.params.editor_state.is_open() {
                // do some processing only when window is open
                if !self.agent_stream.is_empty() {
                    // NOTE: have to use the try methods in relatime thread
                    while let Ok(item) = self.agent_stream.try_recv() {
                        // TODO: talk to maccoy and ask how a DAW
                        // processes audio samples
                    }

                }
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


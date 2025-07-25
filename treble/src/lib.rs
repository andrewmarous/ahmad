use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

mod ui;

pub struct Ahmad {
    params: Arc<AhmadParams>,
}

#[derive(Params)]
struct AhmadParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Ahmad {
    fn default() -> Self {
        Self {
            params: Arc::new(AhmadParams::default()),
        }
    }
}

impl Default for AhmadParams {
    fn default() -> Self {
        Self {
            editor_state: ui::default_state()
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
        ui::create(
            self.params.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for _channel_samples in buffer.iter_samples() {
            // process audio for anything

            if self.params.editor_state.is_open() {
                // process anything more expensive only when ui is open
            }
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Ahmad {
    const VST3_CLASS_ID: [u8; 16] = *b"ahmadfoobarfooba";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_vst3!(Ahmad);

use ndarray::{Array1, Array2};
use midly::{Format, Header, MetaMessage, Smf, Timing, TrackEvent, TrackEventKind};
use bytes::{Bytes, BytesMut};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct NoteEvent {
    pub start_idx: i64,
    pub end_idx: i64,
    pub pitch_midi: i64,
    pub amplitude: f32,
    pub pitch_bends: Option<Vec<i16>>,
}

// TODO: make this consistent with Basic Pitch's ONNX output format (this was taken from
// basicpitch.cpp and needs to be made consistent with custom implementation)
#[derive(Clone, Debug)]
pub struct InferenceResult {
    notes: Array2<i64>,
    onsets: Array2<i64>,
    contours: Array2<i64>,
}

fn output_notes_to_polyphonic(
    preds: &InferenceResult,
    use_melodia_trick: bool,
    include_pitch_bends: bool
) -> Vec<NoteEvent> {
    Vec::new()
}

fn drop_overlapping_pitch_bends(note_events: &mut Vec<NoteEvent>) -> () {

}

fn note_events_to_midi(
    note_events: Vec<NoteEvent>,
    n_times_notes: usize,
) -> Bytes {
    Bytes::new()
}

pub fn convert_to_midi(
    preds: InferenceResult,
    use_melodia_trick: bool,
    include_pitch_bends: bool
) -> Bytes {
    let mut note_events: Vec<NoteEvent> = output_notes_to_polyphonic(
        &preds,
        use_melodia_trick,
        include_pitch_bends
    );
    if include_pitch_bends { drop_overlapping_pitch_bends(&mut note_events); }

    let n_times_notes = preds.notes.dim().0;
    note_events_to_midi(note_events, n_times_notes)
}

// constants
const SAMPLE_RATE: i16 = 22050;
const AUDIO_SAMPLE_RATE: i16 = SAMPLE_RATE;
const FFT_HOP: i16 = 256;
const ANNOTATIONS_FPS: i16 = SAMPLE_RATE / FFT_HOP;
const ONSET_THRESHOLD: f32 =  0.5;
const FRAME_THRESHOLD: f32 =  0.3;
const ANNOTATIONS_BASE_FREQUENCY: f32 =  27.5; // lowest key on a piano
const MAGIC_NUMBER: f32 =  0.0018;
const CONTOURS_BINS_PER_SEMITONE: i8 =  3;
const AUDIO_WINDOW_LENGTH: i8 =  2;
const MIN_NOTE_LEN: i8 =  11;
const MIDI_OFFSET: i8 =  21;
const MAX_FREQ_IDX: i8 =  87;
const ENERGY_TOL: i8 =  11;
const ANNOT_N_FRAMES: f32 = ANNOTATIONS_FPS as f32 * AUDIO_WINDOW_LENGTH as f32;
const AUDIO_N_SAMPLES: f32 = SAMPLE_RATE as f32 * AUDIO_WINDOW_LENGTH as f32 - FFT_HOP as f32;

const MIDI_TEMP_US: i64 = 500_000;
const MIDI_TEMPO_BPM: f32 = 120.0;
const TIME_SIGNATURE_NUMERATOR: i8 = 4;
const TIME_SIGNATURE_DENOMINATOR: i8 = 4;

const DEFAULT_TPQN: i16 = 220;

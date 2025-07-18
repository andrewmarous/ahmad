use nih_plug::prelude::*;
use nih_plug_vizia;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use vizia::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::AhmadParams;

#[derive(Lens)]
struct UserEntryData {
}

#[derive(Lens)]
struct AgentOutputData {
}

#[derive(Lens)]
struct AgentProgressData {
}

#[derive(Lens)]
struct ErrorData {
}


#[derive(Lens)]
struct UIData {
    // plugin state
    params: Arc<AhmadParams>,

    // ui state
    user: UserEntryData,
    output: AgentOutputData,
    progress: AgentProgressData,
    errors: ErrorData,
}

pub enum UIEvent {
    Empty,
    UserEntryEdit(String),
    OutputPathFDSelected,
    OutputNameChanged(String),
    PromptSubmitted,
    AgentProgressUpdated(f32),
    ResponseComplete(String), // TODO: make this bytes and stream straight into DAW
    CheckConnection,
    ConnectionResult(String),
    Reset,
    AgentError(String),
}

impl Model for UIData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            UIEvent::UserEntryEdit(s) => {
            },
            UIEvent::PromptSubmitted => {
            }
            _ => ()
        });
    }
}

use std::io::Bytes;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use futures::StreamExt;
use tracing::{info, error};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use dotenv::dotenv;
use rfd::FileDialog;

use nih_plug::prelude::*;
use nih_plug_vizia;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use vizia::prelude::*;

use crate::AhmadParams;

mod agent;

#[derive(Lens)]
struct UserEntryData {
    content: String,
}

#[derive(Lens)]
struct AgentOutputData {
    filepath: String,
    separator_text: char,
    filename: String,
    progress: f32,
}

#[derive(Lens)]
struct AgentProgressData {
    progress: f32,
}


#[derive(Lens)]
struct UIData {
    // plugin state
    params: Arc<AhmadParams>,

    // ui state
    user: UserEntryData,
    output: AgentOutputData,
    errors: String,
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
    ConnectionResult(Result<(), Error>),
    Reset,
    AgentError(Error),
    AgentInfo(String),
}

impl Default for UserEntryData {
    fn default() -> Self {
        Self {
            content: String::new()
        }
    }
}

impl Default for AgentOutputData {
    fn default() -> Self {
        Self {
            filepath: String::new(),
            separator_text: '/',
            filename: String::new(),
            progress: 0.0,
        }
    }
}

impl Model for UIData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            UIEvent::UserEntryEdit(s) => {
                self.user.content = s.to_owned();
            },
            UIEvent::OutputPathFDSelected => {
                info!("Opening output path file dialog...");
                let filepath = FileDialog::new()
                    .set_directory(
                        std::env::current_dir().unwrap().as_path())
                    .pick_folder()
                    .expect("Error: rfd's pick_folder failed??");
                self.output.filepath = String::from(filepath.to_str().unwrap())
            },
            UIEvent::OutputNameChanged(s) => {
                self.output.filename = s.to_owned();
            },
            UIEvent::PromptSubmitted => {
                info!("User submitted prompt...");
                let filepath: PathBuf = Path::new(self.output.filepath.as_str())
                    .join(self.output.filename.as_str());
                if filepath.exists() {
                    self.errors
                        .push_str(format!("Error: current filepath is not pointing at a valid location. Please
                            ensure that the filepath points to an existing folder that doesn't
                            contain a file named {} \n", self.output.filename).as_str());
                    return ()
                }

                self.output.progress = 0.0;

                Agent::generate(cx, &self.user.content);
            },
            UIEvent::AgentProgressUpdated(f) => {
                self.output.progress = f.to_owned();
            },
            UIEvent::CheckConnection => {
                info!("Checking connection to backend...");
                Agent::check_connection(cx);
            },
            UIEvent::AgentError(e) => {
                error!("Agent error: {e}");
                self.errors = e.to_string();
            }
            UIEvent::AgentInfo(s) => {
                info!("Agent info: {s}");
                self.errors = s.to_owned();
            }
            UIEvent::Reset => {
            }
            _ => ()
        });
    }
}

struct Agent;
impl Agent {
    fn check_connection(cx: &mut EventContext) -> () {
        cx.spawn(|cx| {
            match agent::check_backend() {
                Ok(()) => cx.emit(UIEvent::AgentInfo(
                    String::from("Connected to back-end successfully!"))),
                Err(e) => cx.emit(UIEvent::AgentError(e)),
            };
        });
    }

    fn generate(cx: &mut EventContext, prompt: &String) -> () {
        let p = prompt.clone();
        cx.spawn(|cx| {
            let mut iter = agent::request_response_iterator(p);
            while let Some(se) = iter.next() {
                let se = match se {
                    Ok(a) => a,
                    Err(e) => {
                        cx.emit(UIEvent::AgentError(e));
                        return ();
                    }
                };

                if let Ok(progress) = se.parse::<f32>() {
                    cx.emit(UIEvent::AgentProgressUpdated(progress));
                } else { // bytes have been sent TODO: stream into daw
                    cx.emit(UIEvent::ResponseComplete(String::new()));
                }
            }
        });
    }
}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (400, 200))
}

pub(crate) fn create(
    params: Arc<AhmadParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        UIData {
            params,
            user: UserEntryData::default(),
            output: AgentOutputData::default(),
            errors: String::new(),
        }
        .build(cx);

        // View logic

    })
}

use std::io::Bytes;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
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

pub enum UIEvent<'a> {
    Empty,
    UserEntryEdit(String),
    OutputPathFDSelected,
    OutputNameChanged(String),
    PromptSubmitted,
    AgentProgressUpdated(f32),
    ResponseComplete(&'a String), // TODO: make this bytes and stream straight into DAW
    CheckConnection,
    ConnectionResult(Result<(), Error>),
    Reset,
    AgentError(Error),
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

                let resp = Agent::generate(self.user.content, filepath);
            },
            UIEvent::AgentProgressUpdated(f) => {
                self.output.progress = f.to_owned();
            },
            UIEvent::CheckConnection => {
                info!("Checking connection to backend...");
                match Agent::check_connection() {
                    Ok(_) => info!("Connection successful!"),
                    Err(e) => {
                        error!("Connection unsuccessful: {e}");
                        self.errors = e.to_string();
                    }
                };
            },
            UIEvent::AgentError(e) => {
                error!("Agent error: {e}");
            }
            UIEvent::Reset => {
            }
            _ => ()
        });
    }
}

struct Agent;
impl Agent {
    fn check_connection() -> Result<(), Error> {
        Ok(())
    }

    fn generate(prompt: &String, filepath: Path) {

    }
}

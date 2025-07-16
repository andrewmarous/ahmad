use std::path::PathBuf;

use anyhow::Error;
use iced::widget::{button, column, progress_bar, row, text_editor, text_input, text};
use iced::{Element, Task};
use tracing::{info, error};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;

mod agent;

struct Agent;

#[derive(Default)]
struct UserTextEditor {
    content: text_editor::Content
}

#[derive(Default)]
struct AgentTextInput {
    content: PathBuf
}

#[derive(Default)]
struct AgentProgressBar {
    progress: f32
}

#[derive(Default)]
struct App {
    user: UserTextEditor,
    out_path: AgentTextInput,
    progress: AgentProgressBar,
    errors: String,
}

#[derive(Debug, Clone)]
enum Message {
    UserEdit(text_editor::Action),
    OutputPathChanged(String),
    PromptSubmitted,
    AgentProgressUpdated(f32),
    ResponseComplete(String),
    Reset,
    AgentError,
}

impl UserTextEditor {
    fn new() -> Self {
        Self {
            content: text_editor::Content::new()
        }
    }

    fn update(state: &mut Self, message: Message) {
        match message {
            Message::UserEdit(action) => {
                state.content.perform(action);
            },
            _ => {}
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        text_editor(&state.content)
            .placeholder("Ask for a riff, melody, or bassline with a specific instrument.")
            .on_action(Message::UserEdit)
            .into()
    }
}

impl AgentTextInput {
    fn new() -> Self {
        Self {
            content: PathBuf::new()
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        text_input("path/to/your/output.midi", state.content.to_str().unwrap_or(""))
            .on_input(Message::OutputPathChanged)
            .into()
    }

    fn update(state: &mut Self, message: Message) {
        match message {
            Message::OutputPathChanged(new) => {
                state.content = PathBuf::from(new);
            }
            _ => {}
        }
    }
}

impl AgentProgressBar {
    fn new() -> Self {
        Self {
            progress: 0.0
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        progress_bar(0.0..=100.0, state.progress).into()
    }

    fn update(state: &mut Self, message: Message) {
        match message {
            Message::PromptSubmitted => {
                state.progress = 0.0;
            },
            Message::AgentProgressUpdated(pct) => {
                state.progress = pct;
            },
            _ => {}
        }
    }
}

// need this for Task to work
// impl Default for Agent {
//     fn default() -> Self {
//         Self {
//             task: Task::none(),
//             is_generating: false
//         }
//     }
// }

impl Agent {
    pub fn new() -> Task<Message> {
        Task::run(
            agent::check_backend(),
            move |res | match res {
                Ok(_) => {
                    Message::Reset
                },
                Err(_) => {
                    Message::AgentError
                }
            }
        )
    }

    pub fn request(prompt: String, filepath: PathBuf) -> Task<Message> {
        Task::run(
            agent::request_response_stream(
                prompt.clone(),
                filepath.clone()
            ),
            move |res| match res {
                Ok(s) => {
                    if let Ok(pct) = s.parse() {
                        Message::AgentProgressUpdated(pct)
                    } else {
                        let content = filepath
                            .as_os_str()
                            .to_str()
                            .expect("Output filepath should be valid.");
                        Message::ResponseComplete(content.to_string())
                    }
                }
                Err(_) => {
                    Message::AgentError
                }
            }
        )
    }
}

impl App {
    fn new() -> Self {
        Self {
            user: UserTextEditor::new(),
            out_path: AgentTextInput::new(),
            progress: AgentProgressBar::new(),
            errors: String::from("No errors yet. Happy trails!\n"),
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        column![
            UserTextEditor::view(&state.user),
            AgentTextInput::view(&state.out_path),
            button("Generate").on_press(Message::PromptSubmitted),
            AgentProgressBar::view(&state.progress),
            text(&state.errors[..]).size(20)
        ]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn update(state: &mut Self, message: Message)  -> Task<Message> {
        match message {
            Message::UserEdit(s) => {
                info!("user edited model prompt.");
                UserTextEditor::update(&mut state.user, Message::UserEdit(s));
                Task::none()
            },
            Message::OutputPathChanged(s) => {
                info!("user changed output path: {}", s);
                AgentTextInput::update(&mut state.out_path, Message::OutputPathChanged(s));
                Task::none()
            },
            Message::AgentProgressUpdated(f) => {
                info!("agent progress updated to {}", f);
                AgentProgressBar::update(&mut state.progress, Message::AgentProgressUpdated(f));
                Task::none()
            },
            Message::PromptSubmitted => {
                state.errors.clear();
                info!("prompt submitted...");
                // check for errors
                // state.errors.clear();
                // let Some(_) = state.out_path.content.to_str() else {
                //     state.errors.push_str("Error: no output filepath defined. Please define where
                //                            you'd like the generated MIDI to go.\n");
                //     return Task::none()
                // };

                if !state.out_path.content.is_dir() {
                    let filename = match state.out_path.content.file_name() {
                        Some(name) => name.to_str().unwrap_or(""),
                        None => "",
                    };
                    info!("Pushing error string to state.errors...");
                    state.errors
                        .push_str(format!("Error: current filepath is not pointing at a valid location. Please
                                           ensure that the filepath points to an existing folder that doesn't
                                           contain a file named {} \n", filename).as_str());
                    return Task::none();
                }

                AgentProgressBar::update(&mut state.progress, Message::AgentProgressUpdated(50.0));
                Agent::request(
                    state.user.content.text().to_owned(),
                    state.out_path.content.to_owned(),
                )
            },
            Message::AgentError => {
                state.errors.clear();
                state.errors.push_str("Error: agent failed generation. Check dev logs for more detail.\n");
                Task::none()
            },
            Message::ResponseComplete(user_msg) => {
                state.errors.clear();
                state.errors.push_str(&user_msg);
                Task::none()
            },
            Message::Reset => {
                state.errors.clear();
                state.user.content = text_editor::Content::new();
                state.out_path.content = PathBuf::new();
                Task::none()
            },
        }
    }

    // fn subscription(&self) -> Subscription<Message> {
    //     if self.agent.is_generating {
    //         Subscription::run(
    //             agent::agent::request_response_stream(
    //                 self.user.content.text().to_owned(),
    //                 self.out_path.content.to_owned()
    //             )
    //         )
    //     } else {
    //         Subscription::none()
    //     }
    // }
}

pub fn main() -> iced::Result {
    let file_appender: RollingFileAppender = tracing_appender::rolling::daily("logs", "plugin.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .init();

    info!("Starting UI...");
    iced::application("ahmad 0.1a.0", App::update, App::view)
        .run_with( || (App::new(), Agent::new()))
}

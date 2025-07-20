// TODO: port this entire fucking thing to vizia

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use nih_plug::prelude::GuiContext;
use anyhow::Error;
use tracing::{info, error};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use dotenv::dotenv;
use rfd::FileDialog;

use iced::{Element, Length, Task, executor};
use iced::widget::{button, column, container, progress_bar, row, text, text_editor, text_input};
use nih_plug_iced::widgets as nih_widgets;
use nih_plug_iced::IcedEditor;

use crate::AhmadParams;

mod agent;

struct Agent;

#[derive(Default)]
struct UserTextEditor {
    content: text_editor::Content
}

#[derive(Default)]
struct AgentOutputContainer {
    filepath: String,
    separator_text: String,
    filename: String,
}

#[derive(Default)]
struct AgentProgressBar {
    progress: f32
}

struct AhmadEditor {
    // plugin state
    params: Arc<AhmadParams>,
    context: Arc<dyn GuiContext>,

    // ui state
    user: UserTextEditor,
    out_path: AgentOutputContainer,
    progress: AgentProgressBar,
    errors: String,
}

#[derive(Debug, Clone, Default)]
enum Message {
    // plugin messages
    ParamUpdate(nih_widgets::ParamMessage),

    // ui messages
    #[default]
    Empty,
    UserEdit(text_editor::Action),
    OutputPathFDSelected,
    OutputNameChanged(String),
    PromptSubmitted,
    AgentProgressUpdated(f32),
    ResponseComplete(String),
    CheckConnection,
    ConnectionResult(String),
    Reset,
    AgentError(String),
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
            .placeholder(
                "Ask for a riff, melody, or bassline with a specific instrument. Be sure to include descriptive adjectives and adjectives like 'high quality' or 'clear'."
            )
            .on_action(Message::UserEdit)
            .into()
    }
}

impl AgentOutputContainer {
    fn new() -> Self {
        Self {
            filepath: String::from("Select a folder..."),
            separator_text: String::from("/"),
            filename: String::new(),
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        container(
            column![
                text("Output path:").size(10),
                row![
                    button(&state.filepath[..])
                        .on_press(Message::OutputPathFDSelected)
                        .width(Length::FillPortion(3)),
                    text(&state.separator_text).size(20),
                    text_input("example.midi or example.wav", &state.filename[..])
                        .on_input(Message::OutputNameChanged)
                        .width(Length::FillPortion(1))
                ]
                    .spacing(10)
                    .padding(20)
            ]
                .padding(10)
        )
            .align_left(Length::Shrink)
            .style(container::rounded_box)
            .into()
    }

    fn update(state: &mut Self, message: Message) {
        match message {
            Message::OutputPathFDSelected => {
                // init RFD session, pull value from that and assign to state.filepath
                let filepath = FileDialog::new()
                    .set_directory(
                        std::env::current_dir().unwrap().as_path())
                    .pick_folder()
                    .expect("Error: rfd's pick_folder failed??");
                state.filepath = String::from(filepath.to_str().unwrap())
            },
            Message::OutputNameChanged(s) => {
                state.filename = String::from(s);
            },
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

    pub fn reset() -> Task<Message> { Task::done(Message::Reset) }

    pub fn check_connection() -> Task<Message> {
        Task::run(
            agent::check_backend(),
            move |res | match res {
                Ok(_) => {
                    Message::ConnectionResult(
                        String::from("Connection to AI backend is successful!")
                    )
                },
                Err(e) => {
                    error!("Error checking connection to backend: {e}");
                    Message::ConnectionResult(e.to_string())
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
                        let size: usize = s.parse().unwrap();
                        Message::ResponseComplete(String::from(
                            format!("Model response received, file is {} bytes", size)
                        ))
                    }
                }
                Err(e) => {
                    Message::AgentError(e.to_string())
                }
            }
        )
    }
}

impl IcedEditor for AhmadEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = Arc<AhmadParams>;

    fn new(
        params: Arc<AhmadParams>, context: Arc<dyn GuiContext>
    ) -> (Self, Task<Self::Message>) {
        let editor = Self {
            params,
            context,

            user: UserTextEditor::new(),
            out_path: AgentOutputContainer::new(),
            progress: AgentProgressBar::new(),
            errors: String::from("No errors yet. Happy trails!\n"),
        };
        (editor, Task::none())
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        column![
            UserTextEditor::view(*self.user),
            AgentOutputContainer::view(&self.out_path),
            button("Check Connection").on_press(Message::CheckConnection),
            button("Generate").on_press(Message::PromptSubmitted),
            AgentProgressBar::view(&self.progress),
            text(self.errors[..]).size(20)
        ]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn update(&mut self, _window: &mut WindowQueue, message: Message)  -> Task<Message> {
        match message {
            Message::UserEdit(s) => {
                info!("user edited model prompt.");
                UserTextEditor::update(&mut self.user, Message::UserEdit(s));
            },
            Message::OutputNameChanged(s) => {
                info!("user changed output filename: {}", s);
                AgentOutputContainer::update(&mut self.out_path, Message::OutputNameChanged(s));
            },
            Message::OutputPathFDSelected => {
                info!("opening folder select dialog...");
                AgentOutputContainer::update(&mut self.out_path, Message::OutputPathFDSelected);
            }
            Message::AgentProgressUpdated(f) => {
                info!("agent progress updated to {}", f);
                AgentProgressBar::update(&mut self.progress, Message::AgentProgressUpdated(f));
            },
            Message::PromptSubmitted => {
                self.errors.clear();
                info!("prompt submitted...");
                // check for errors
                // self.errors.clear();
                // let Some(_) = self.out_path.content.to_str() else {
                //     self.errors.push_str("Error: no output filepath defined. Please define where
                //                            you'd like the generated MIDI to go.\n");
                //     return Task::none()
                // };

                // construct output filepath
                let filepath: PathBuf = Path::new(self.out_path.filepath.as_str())
                    .join(self.out_path.filename.as_str());

                if filepath.exists() {
                    self.errors
                        .push_str(format!("Error: current filepath is not pointing at a valid location. Please
                                           ensure that the filepath points to an existing folder that doesn't
                                           contain a file named {} \n", self.out_path.filename).as_str());
                    return Task::none();
                }

                AgentProgressBar::update(&mut self.progress, Message::AgentProgressUpdated(0.0));
                return Agent::request(
                    self.user.content.text().to_owned(),
                    filepath
                )
            },
            Message::AgentError(e) => {
                error!("Error with agent: {e}");
                self.errors.clear();
                let fmtstr = format!("Error generating response: {}", e);
                self.errors.push_str(&fmtstr);
            },
            Message::ResponseComplete(user_msg) => {
                self.errors.clear();
                self.errors.push_str(&user_msg);
            },
            Message::Reset => {
                self.errors.clear();
                self.user = UserTextEditor::new();
                self.out_path = AgentOutputContainer::new();
            },
            Message::CheckConnection => {
                self.errors.clear();
                Agent::check_connection()
            },
            Message::ConnectionResult(s) => {
                self.errors.clear();
                self.errors.push_str(&s[..]);
            },
            Message::ParamUpdate(message) => self.handle_param_message(message),
            Message::Empty => {}
        }
        Task::none()
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

// pub fn main() -> iced::Result {
//     dotenv().ok();
//     let file_appender: RollingFileAppender = tracing_appender::rolling::daily("logs", "plugin.log");
//     let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
//     tracing_subscriber::fmt()
//         .with_max_level(tracing::Level::INFO)
//         .with_writer(non_blocking)
//         .init();

//     info!("Starting UI...");
//     iced::application("ahmad 0.1a.0", App::update, App::view)
//         .run_with( || (App::new(), Agent::reset()))
// }
//

pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(400, 200)
}

pub(crate) fn create(
    params: Arc<AhmadParams>,
    editor_state: Arc<IcedState>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<AhmadEditor>(editor_state, params)
}

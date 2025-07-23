use std::path::{Path, PathBuf};
use std::sync::Arc;

use nih_plug::prelude::GuiContext;
use nih_plug::wrapper::vst3::Wrapper;
use nih_plug::{nih_error, nih_log, wrapper};

use anyhow::Error;
use dotenv::dotenv;
use rfd::FileDialog;

use iced_baseview::{window, window::WindowSubs, futures::Subscription, Element, Length, Task, Size, Application};
use iced_baseview::widget::{button, column, container, progress_bar, row, text, text_editor, text_input};
use iced_baseview::core as iced;

use crate::editor::{agent, UIFlags};

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

#[derive(Default)]
pub struct UIState {
    // context: Arc<dyn GuiContext>,
    user: UserTextEditor,
    out_path: AgentOutputContainer,
    progress: AgentProgressBar,
    errors: String,
}

#[derive(Debug, Clone, Default)]
pub enum Message {
    #[default]
    Empty,
    Window(iced::window::Event),
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
                    nih_error!("Error checking connection to backend: {e}");
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

impl Application for UIState {
    type Message = Message;
    type Theme = iced_baseview::Theme;
    type Executor = iced_baseview::executor::Default;
    type Flags = UIFlags;

    fn new(_flags: Self::Flags) -> (Self, Task<Self::Message>) {
        dotenv().ok();
        (Self {
            // context: flags.context,
            user: UserTextEditor::new(),
            out_path: AgentOutputContainer::new(),
            progress: AgentProgressBar::new(),
            errors: String::from("No errors yet. Happy trails!\n"),
        }, Task::none() )
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            UserTextEditor::view(&self.user),
            AgentOutputContainer::view(&self.out_path),
            button("Check Connection").on_press(Message::CheckConnection),
            button("Generate").on_press(Message::PromptSubmitted),
            AgentProgressBar::view(&self.progress),
            text(&self.errors[..]).size(20)
        ]
            .spacing(20)
            .padding(20)
            .into()
    }


    fn update(&mut self, message: Message)  -> Task<Message> {
        match message {
            Message::Window(event) => {
                // match event {
                //     iced::window::Event::Resized(_) => {
                //         iced::window::RedrawRequest;
                //         self.context.request_resize(); }
                //     _ => {}
                // }
            }
            Message::UserEdit(s) => {
                nih_log!("user edited model prompt.");
                UserTextEditor::update(&mut self.user, Message::UserEdit(s));
            },
            Message::OutputNameChanged(s) => {
                nih_log!("user changed output filename: {}", s);
                AgentOutputContainer::update(&mut self.out_path, Message::OutputNameChanged(s));
            },
            Message::OutputPathFDSelected => {
                nih_log!("opening folder select dialog...");
                AgentOutputContainer::update(&mut self.out_path, Message::OutputPathFDSelected);
            }
            Message::AgentProgressUpdated(f) => {
                nih_log!("agent progress updated to {}", f);
                AgentProgressBar::update(&mut self.progress, Message::AgentProgressUpdated(f));
            },
            Message::PromptSubmitted => {
                self.errors.clear();
                nih_log!("prompt submitted...");
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
                );
            },
            Message::AgentError(e) => {
                nih_error!("Error with agent: {e}");
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
                return Agent::check_connection();
            },
            Message::ConnectionResult(s) => {
                self.errors.clear();
                self.errors.push_str(&s[..]);
            },
            Message::Empty => {}
        }
        Task::none()
    }

    fn theme(&self) -> Self::Theme {
       iced::Theme::Dark
    }

    fn style(&self, theme: &Self::Theme) -> iced_baseview::Appearance {
        iced_baseview::Appearance {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        }
    }

    // fn subscription(&self, window_subs: &mut WindowSubs<Message>) -> Subscription<Message> {
    //     iced_baseview::window::events().map(Message::Window)
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

//     nih_log!("Starting UI...");
//     iced::application("ahmad 0.1a.0", App::update, App::view)
//         .run_with( || (App::new(), Agent::reset()))
// }

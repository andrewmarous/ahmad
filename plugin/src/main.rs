use std::ffi::OsString;
use std::error::Error;
use std::path::{self, PathBuf};

use futures::Stream;
use iced::widget::{button, column, progress_bar, row, text_editor, text_input};
use iced::{Element, Task, Subscription};

pub mod agent;

struct Agent {
    task: Task<Message>,
    is_generating: bool
}

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
    agent: Agent
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
impl Default for Agent {
    fn default() -> Self {
        Self {
            task: Task::none(),
            is_generating: false
        }
    }
}

impl Agent {
    pub fn new() -> () {
        match agent::agent::initialize() {
            Ok(_) => (),
            Err(e) => panic!("Error initializing model: {}", e.to_string().as_str())
        }
    }

    pub fn request(&mut self, prompt: String, filepath: PathBuf) -> () {
        self.task = Task::run(
            agent::agent::request_response_stream(
                prompt.clone(),
                filepath.clone()
            ),
            move |res| match res {
                Ok(s) => {
                    // if let Ok(pct) = s.parse() {
                    //     Message::AgentProgressUpdated(pct)
                    // } else {
                    //     let content = filepath
                    //         .as_os_str()
                    //         .to_str()
                    //         .expect("Output filepath should be valid.");
                    //     let fmtstr = format!("Generated MIDI successfully saved to: {}", content);
                    //     Message::ResponseComplete(String::from(fmtstr.as_str()))
                    // }
                    let fmtstr = format!("Generated MIDI successfully saved to: {}", s);
                    Message::ResponseComplete(fmtstr.to_owned())
                }
                Err(_) => {
                    Message::AgentError
                }
            }
        );
    }
}

impl App {
    fn new() -> Self {
        Self {
            user: UserTextEditor::new(),
            out_path: AgentTextInput::new(),
            progress: AgentProgressBar::new(),
            errors: String::from("No errors yet. Happy trails!"),
            agent: Agent::default()
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        column![
            UserTextEditor::view(&state.user),
            AgentTextInput::view(&state.out_path),
            button("Generate").on_press(Message::PromptSubmitted),
            AgentProgressBar::view(&state.progress),
        ]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn update(state: &mut Self, message: Message)  -> Task<Message> {
        match message {
            Message::UserEdit(s) => {
                UserTextEditor::update(&mut state.user, Message::UserEdit(s));
                Task::none()
            },
            Message::OutputPathChanged(s) => {
                AgentTextInput::update(&mut state.out_path, Message::OutputPathChanged(s));
                Task::none()
            },
            Message::AgentProgressUpdated(f) => {
                AgentProgressBar::update(&mut state.progress, Message::AgentProgressUpdated(f));
                Task::none()
            },
            Message::PromptSubmitted => {
                // check for errors
                state.errors.clear();
                let Some(filepath) = state.out_path.content.to_str() else {
                    state.errors.push_str("Error: no output filepath defined. Please define where
                                           you'd like the generated MIDI to go.\n");
                    return Task::none()
                };

                if !state.out_path.content.is_dir() {
                    let filename = match state.out_path.content.file_name() {
                        Some(name) => name.to_str().unwrap_or(""),
                        None => "",
                    };
                    state.errors
                        .push_str(format!("Error: current filepath is not pointing at a valid location. Please
                                           ensure that the filepath points to an existing folder that doesn't
                                           contain a file named {} \n", filename).as_str());
                    return Task::none();
                }

                AgentProgressBar::update(&mut state.progress, Message::AgentProgressUpdated(50.0));
                state.agent.request(
                    state.user.content.text().to_owned(),
                    state.out_path.content.to_owned());
                Task::none()

            },
            _ => { Task::none() }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.agent.is_generating {
            Subscription::run(
                agent::agent::request_response_stream(
                    self.user.content.text().to_owned(),
                    self.out_path.content.to_owned()
                )
            )
        } else {
            Subscription::none()
        }
    }
}

pub fn main() -> iced::Result {
    iced::application("ahmad 0.1a.0", App::update, App::view)
        .subscription(App::subscription)
        .run_with(App::new())
}

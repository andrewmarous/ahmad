use std::ffi::OsString;
use std::path::{self, PathBuf};

use iced::widget::{button, column, progress_bar, row, text_editor, text_input};
use iced::{Element, Task};

pub mod agent;

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
    errors: String
}

#[derive(Debug, Clone)]
enum Message {
    UserEdit(text_editor::Action),
    OutputPathChanged(String),
    PromptSubmitted,
    AgentProgressUpdated(f32),
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
        text_input("path/to/your/output_folder", state.content.to_str().unwrap_or(""))
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

impl App {
    fn new() -> Self {

        Self {
            user: UserTextEditor::new(),
            out_path: AgentTextInput::new(),
            progress: AgentProgressBar::new(),
            errors: String::from("No errors yet. Happy trails!"),
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

    fn update(state: &mut Self, message: Message) {
        match message {
            Message::UserEdit(s) => {
                UserTextEditor::update(&mut state.user, Message::UserEdit(s));
            },
            Message::OutputPathChanged(s) => {
                AgentTextInput::update(&mut state.out_path, Message::OutputPathChanged(s));
            },
            Message::AgentProgressUpdated(f) => {
                AgentProgressBar::update(&mut state.progress, Message::AgentProgressUpdated(f));
            },
            Message::PromptSubmitted => {
                // check for errors
                state.errors.clear();
                let Some(filepath) = state.out_path.content.to_str() else {
                    state.errors.push_str("Error: no output filepath defined. Please define where
                                           you'd like the generated MIDI to go.\n");
                    return;
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
                    return;
                }

                Task::run(
                    agent::request_response(
                        &state.user.content.text(),
                        &state.out_path.content
                    ),
                    |res| match res {
                        Ok(pct) => Message::AgentProgressUpdated(pct),
                        Err(e) => {
                                state.errors.push_str(e);
                                Message::AgentError
                        }
                    }
                )
            },
            _ => {}
        }
    }
}

pub fn main() -> iced::Result {
    // initialize chatboxes and labels

    // run UI elements
    iced::run("", ModelInterface::update, ModelInterface::view)
}

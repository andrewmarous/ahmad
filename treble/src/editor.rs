use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex, atomic::Ordering};

use bytes::Bytes;
use crossbeam::channel::{self, Sender};
use nih_plug::prelude::{GuiContext, ParamPtr};
use nih_plug::{nih_error, nih_log, wrapper};

use anyhow::Error;
use dotenv::dotenv;
use num_enum::TryFromPrimitive;
use rfd::FileDialog;

use iced_baseview::futures::backend::native::smol::time::every;
use iced_baseview::{window::WindowSubs, futures::Subscription, Element, Length, Task, Size, Application};
use iced_baseview::widget::{button, column, container, progress_bar, row, text, text_editor, text_input};
use iced_baseview::{core as iced, executor};

use crate::libplugui::{IcedEditor, ParamMessage, create_iced_editor, IcedState};
use crate::{AhmadParams, ResponseFiletype};

mod agent;

struct Agent {
    sender: Arc<Sender<Result<Bytes, Error>>>
}

#[derive(Default)]
struct UserTextInput {
    content: String
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

pub struct AhmadEditor {
    // plugin fields
    context: Arc<dyn GuiContext>,
    params: Arc<AhmadParams>,

    // ui fields
    user: UserTextInput,
    agent: Agent,
    out_path: AgentOutputContainer,
    progress: AgentProgressBar,
    errors: String,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Default)]
pub enum Message {
    #[default]
    Empty,
    UserEdit(String),
    WindowResized,
    // UserEdit(text_editor::Action),
    OutputPathFDSelected,
    OutputNameChanged(String),
    PromptSubmitted,
    AgentProgressUpdated(f32),
    ResponseComplete(String),
    CheckConnection,
    ConnectionResult(String),
    Reset,
    AgentError(String),
    // Add parameter message handling
    Param(ParamMessage),
}

impl UserTextInput {
    fn new() -> Self {
        Self {
            content: String::new()
        }
    }

    fn update(state: &mut Self, message: Message) {
        match message {
            Message::UserEdit(string) => {
                state.content = string
            },
            _ => {}
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        // let text = state.content.lock().unwrap().text();
        // let mut new_content = text_editor::Content::new();
        // new_content.perform(
        //     text_editor::Action::Edit(
        //         text_editor::Edit::Paste(
        //             Arc::new(String::from(text)))));

        // TODO: try to find a way to make this a thread-safe text editor
        let editor = text_input(&"Ask for a riff, melody, or bassline with a specific instrument. Be sure to include descriptive adjectives and adjectives like 'high quality' or 'clear'.",
            &state.content)
            .on_input(Message::UserEdit)
            .into();
        editor
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
        text("Agent placeholder...").into()
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
    pub fn new(sender: Arc<Sender<Result<Bytes, Error>>>) -> Self {
        Self {
           sender
        }
    }

    // TODO: move this to AhmadEditor
    pub fn reset() -> Task<Message> { Task::done(Message::Reset) }

    pub fn check_connection(&self) -> Task<Message> {
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

    pub fn request(&self, prompt: String, filetype: ResponseFiletype) -> Task<Message> {
        Task::run(
            agent::request_response_stream(
                prompt.clone(),
                self.sender.clone(),
                filetype,
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
    type Executor = iced_baseview::executor::Default;
    type Message = Message;
    type InitializationFlags = (Arc<AhmadParams>, Arc<Sender<Result<Bytes, Error>>>); // Pass params as initialization flags

    fn new(
        params: Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Task<Self::Message>) {
        let (w, h) = params.0.editor_state.size();
        (
            Self {
                context,
                params: params.0,
                user: UserTextInput::new(),
                agent: Agent::new(params.1),
                out_path: AgentOutputContainer::new(),
                progress: AgentProgressBar::new(),
                errors: String::from("No errors yet. Happy trails!\n"),
                width: w as f32,
                height: h as f32,
            },
            Task::none()
        )
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    #[inline]
    fn update(
        &mut self,
        message: Self::Message,
    ) -> Task<Self::Message> {

        match message {
            Message::WindowResized => {
                let (w, h) = self.params.editor_state.size();
                nih_log!("ui resizing window to {}x{}", w, h);
                // iced_baseview::window::resize(Size {
                //     width: w as f32,
                //     height: h as f32,
                // })
                self.width = w as f32; self.height = h as f32;
                Task::none()
            },
            Message::UserEdit(s) => {
                nih_log!("user edited model prompt.");
                UserTextInput::update(&mut self.user, Message::UserEdit(s));
                Task::none()
            },
            Message::OutputNameChanged(s) => {
                nih_log!("user changed output filename: {}", s);
                AgentOutputContainer::update(&mut self.out_path, Message::OutputNameChanged(s));
                Task::none()
            },
            Message::OutputPathFDSelected => {
                // FIX: take this out when data streaming works
                nih_log!("opening folder select dialog...");
                AgentOutputContainer::update(&mut self.out_path, Message::OutputPathFDSelected);
                Task::none()
            }
            Message::AgentProgressUpdated(f) => {
                nih_log!("agent progress updated to {}", f);
                AgentProgressBar::update(&mut self.progress, Message::AgentProgressUpdated(f));
                Task::none()
            },
            Message::PromptSubmitted => {
                self.errors.clear();
                nih_log!("prompt submitted...");

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
                // request is initiated and handled with an Iced Task
                return self.agent.request(
                    self.user.content.clone(),
                    ResponseFiletype::try_from_primitive(
                        self.params.filetype.load(Ordering::Relaxed))
                        .expect("filetype should be a valid enum variant")
                );
            },
            Message::AgentError(e) => {
                nih_error!("Error with agent: {e}");
                self.errors.clear();
                let fmtstr = format!("Error generating response: {}", e);
                self.errors.push_str(&fmtstr);
                Task::none()
            },
            Message::ResponseComplete(user_msg) => {
                self.errors.clear();
                self.errors.push_str(&user_msg);
                Task::none()
            },
            Message::Reset => {
                self.errors.clear();
                self.user = UserTextInput::new();
                self.out_path = AgentOutputContainer::new();
                Task::none()
            },
            Message::CheckConnection => {
                self.errors.clear();
                return self.agent.check_connection();
            },
            Message::ConnectionResult(s) => {
                self.errors.clear();
                self.errors.push_str(&s[..]);
                Task::none()
            },
            Message::Param(param_message) => {
                // Handle parameter messages using the trait method
                self.handle_param_message(param_message);
                Task::none()
            },
            Message::Empty => {
                Task::none()
            },
        }
    }

    fn view(&self) -> Element<Self::Message> {
        column![
            UserTextInput::view(&self.user),
            AgentOutputContainer::view(&self.out_path),
            button("Check Connection").on_press(Message::CheckConnection),
            button("Generate").on_press(Message::PromptSubmitted),
            AgentProgressBar::view(&self.progress),
            text(&self.errors[..]).size(20)
        ]
            .spacing(20)
            .padding(20)
            .width(Length::Fixed(self.width - 20.0))
            .height(Length::Fixed(self.height - 20.0))
            .into()

    }
    fn subscription(&self, _window_subs: &mut WindowSubs<Self::Message>) -> Subscription<Self::Message> {
        // TODO: add window event subscription? Does this even need to be handled here?
        // Maybe put this in the application wrapper?
        let (w, h) = self.params.editor_state.size();
        let sw = self.width;
        let sh = self.height;
        every(iced::time::Duration::from_millis(200))
            .map(move |_| {
                let (w, h) = (w as f32, h as f32);
                if w != sw || h != sh {
                    Message::WindowResized
                } else {
                    Message::Empty
                }
            })
    }
}

pub fn create(params: Arc<AhmadParams>, tx: Arc<Sender<Result<Bytes, Error>>>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
    create_iced_editor::<AhmadEditor> (
        params.editor_state.clone(),
        (params, tx),
    )
}

pub fn default_state() -> Arc<IcedState> {
    IcedState::from_size(800, 400)
}

// struct TestEditor {
//     context: Arc<dyn GuiContext>,
//     test: i32,
// }

// #[derive(Debug, Clone)]
// enum TestMessage {
//     Message
// }

// impl IcedEditor for TestEditor {
//     type Executor = iced_baseview::executor::Default;
//     type Message = TestMessage;
//     type InitializationFlags = Arc<AhmadParams>; // Pass params as initialization flags

//     fn new(
//         params: Self::InitializationFlags,
//         context: Arc<dyn GuiContext>,
//     ) -> (Self, Task<Self::Message>) {
//         (
//             Self {
//                 context,
//                 test: 0
//             },
//             Task::none()
//         )
//     }

//     fn context(&self) -> &dyn GuiContext {
//         self.context.as_ref()
//     }

//     #[inline]
//     fn update(
//         &mut self,
//         message: Self::Message,
//     ) -> Task<Self::Message> {
//         match message {
//             TestMessage::Message => Task::none(),
//         }
//     }

//     fn view(&self) -> Element<Self::Message> {
//         button("Test123").into()
//     }
//     fn subscription(&self, _window_subs: &mut WindowSubs<Self::Message>) -> Subscription<Self::Message> {
//         // TODO: add window event subscription? Does this even need to be handled here?
//         // Maybe put this in the application wrapper?
//         Subscription::none()
//     }
// }


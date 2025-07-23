use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam::channel;
use nih_plug::prelude::{GuiContext, ParamPtr};
use nih_plug::{nih_error, nih_log, wrapper};

use anyhow::Error;
use dotenv::dotenv;
use rfd::FileDialog;

use iced_baseview::{window, window::WindowSubs, futures::Subscription, Element, Length, Task, Size, Application};
use iced_baseview::widget::{button, column, container, progress_bar, row, text, text_editor, text_input};
use iced_baseview::core as iced;

use crate::libplugui::{IcedEditor, ParameterUpdate};

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

#[derive(Default)]
pub struct AhmadWrapperApplication<E: IcedEditor> {
    // plugin fields
    editor: E,
    parameter_updates_receiver: Arc<channel::Receiver<ParameterUpdate>>,

    // ui fields
    user: UserTextEditor,
    out_path: AgentOutputContainer,
    progress: AgentProgressBar,
    errors: String,
}

pub enum Message<E: IcedEditor> {
    EditorMessage(E::Message),
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

impl<E: IcedEditor> std::fmt::Debug for Message<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // FIX: fill this out
            _ => write!(f, "Debug not implemented on this message yet"),
        }
    }
}

impl<E: IcedEditor> Clone for Message<E> {
    fn clone(&self) -> Self {
        match self {
            // FIX: fill this out
            _ => Self::Reset
        }
    }
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

impl<E: IcedEditor> Application for AhmadWrapperApplication<E> {
    type Message = Message<E>;
    type Theme = iced_baseview::Theme;
    type Executor = iced_baseview::executor::Default;
    type Flags = (
        Arc<dyn GuiContext>,
        Arc<channel::Receiver<ParameterUpdate>>,
        E::InitializationFlags,
    );

    fn new( (context, parameter_updates_receiver, flags): Self::Flags,
    ) -> (Self, Task<Self::Message>) {
        let (editor, task) = E::new(flags, context);
        (
            Self {
                editor,
                parameter_updates_receiver,
                user: UserTextEditor::new(),
                out_path: AgentOutputContainer::new(),
                progress: AgentProgressBar::new(),
                errors: String::from("No errors yet. Happy trails!\n"),
            },
            Task::none()
        )
    }

    #[inline]
    fn view(&self) -> Element<'_, Message<E>> {
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


    #[inline]
    fn update(&mut self, message: Message<E>)  -> Task<Message<E>> {
        match message {
            Message::Window(event) => {
                // FIX: this needs to call GUI context and resize window
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

    #[inline]
    fn subscription(
        &self,
        window_subs: &mut WindowSubs<Self::Message>,
    ) -> Subscription<Self::Message> {
        // Since we're wrapping around `E::Message`, we need to do this transformation ourselves
        let mut editor_window_subs = WindowSubs {
            on_frame: match &window_subs.on_frame {
                Some(Message::EditorMessage(message)) => Some(message.clone()),
                _ => None,
            },
            on_window_will_close: match &window_subs.on_window_will_close {
                Some(Message::EditorMessage(message)) => Some(message.clone()),
                _ => None,
            },
        };

        let subscription = Subscription::batch([
            // For some reason there's no adapter to just convert `futures::channel::mpsc::Receiver`
            // into a stream that doesn't require consuming that receiver (which wouldn't work in
            // this case since the subscriptions function gets called repeatedly). So we'll just use
            // a crossbeam queue and this unfold instead.
            subscription::unfold(
                "parameter updates",
                self.parameter_updates_receiver.clone(),
                |parameter_updates_receiver| match parameter_updates_receiver.try_recv() {
                    Ok(_) => futures::future::ready((
                        Some(Message::ParameterUpdate),
                        parameter_updates_receiver,
                    ))
                    .boxed(),
                    Err(_) => futures::future::pending().boxed(),
                },
            ),
            self.editor
                .subscription(&mut editor_window_subs)
                .map(Message::EditorMessage),
        ]);

        if let Some(message) = editor_window_subs.on_frame {
            window_subs.on_frame = Some(Message::EditorMessage(message));
        }
        if let Some(message) = editor_window_subs.on_window_will_close {
            window_subs.on_window_will_close = Some(Message::EditorMessage(message));
        }

        subscription
    }

    #[inline]
    fn theme(&self) -> Self::Theme {
        iced::Theme::Dark
    }

    #[inline]
    fn title(&self) -> String {
        String::from(env!("CARGO_PKG_NAME"))
    }

    #[inline]
    fn style(&self, theme: &Self::Theme) -> iced_baseview::Appearance {
        iced_baseview::Appearance {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        }
    }

}


#[derive(Debug, Clone, Copy)]
pub enum ParamMessage {
    /// Begin an automation gesture for a parameter.
    BeginSetParameter(ParamPtr),
    /// Set a parameter to a new normalized value. This needs to be surrounded by a matching
    /// `BeginSetParameter` and `EndSetParameter`.
    SetParameterNormalized(ParamPtr, f32),
    /// End an automation gesture for a parameter.
    EndSetParameter(ParamPtr),
}


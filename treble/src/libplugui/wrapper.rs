
//! An [`Application`] wrapper around an [`IcedEditor`] to bridge between `iced_baseview` and
//! `nih_plug_iced`.

use crossbeam::channel;
use nih_plug::prelude::{GuiContext, ParamPtr};
use std::sync::Arc;

use futures::{FutureExt, StreamExt};
// TODO: find subscription module
use iced::{
    futures, Color, Task, Element,
    Subscription,
};

use iced_baseview::futures::subscription;
use iced_baseview::{window::{WindowQueue, WindowSubs}, Application};
use baseview::WindowScalePolicy;
use crate::libplugui::{IcedEditor, ParameterUpdate};

/// Wraps an `iced_baseview` [`Application`] around [`IcedEditor`]. Needed to allow editors to
/// always receive a copy of the GUI context.
pub(crate) struct IcedEditorWrapperApplication<E: IcedEditor> {
    editor: E,

    /// We will receive notifications about parameters being changed on here. Whenever a parameter
    /// update gets sent, we will trigger a [`Message::parameterUpdate`] which causes the UI to be
    /// redrawn.
    parameter_updates_receiver: Arc<channel::Receiver<ParameterUpdate>>,
}

/// This wraps around `E::Message` to add a parameter update message which can be handled directly
/// by this wrapper. That parameter update message simply forces a redraw of the GUI whenever there
/// is a parameter update.
pub enum Message<E: IcedEditor> {
    EditorMessage(E::Message),
    ParameterUpdate,
}

impl<E: IcedEditor> std::fmt::Debug for Message<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EditorMessage(arg0) => f.debug_tuple("EditorMessage").field(arg0).finish(),
            Self::ParameterUpdate => write!(f, "ParameterUpdate"),
        }
    }
}

impl<E: IcedEditor> Clone for Message<E> {
    fn clone(&self) -> Self {
        match self {
            Self::EditorMessage(arg0) => Self::EditorMessage(arg0.clone()),
            Self::ParameterUpdate => Self::ParameterUpdate,
        }
    }
}

impl<E: IcedEditor> Application for IcedEditorWrapperApplication<E> {
    type Executor = E::Executor;
    type Theme = iced::Theme;
    type Message = Message<E>;
    type Flags = (
        Arc<dyn GuiContext>,
        Arc<channel::Receiver<ParameterUpdate>>,
        E::InitializationFlags,
    );

    fn new(
        (context, parameter_updates_receiver, flags): Self::Flags,
    ) -> (Self, Task<Self::Message>) {
        let (editor, command) = E::new(flags, context);

        (
            Self {
                editor,
                parameter_updates_receiver,
            },
            command.map(Message::EditorMessage),
        )
    }

    #[inline]
    fn update(
        &mut self,
        message: Self::Message,
    ) -> Task<Self::Message> {
        match message {
            Message::EditorMessage(message) => self
                .editor
                .update(message)
                .map(Message::EditorMessage),
            // This message only exists to force a redraw
            Message::ParameterUpdate => Task::none(),
        }
    }

    #[inline]
    fn subscription(
        &self,
        window_subs: &mut WindowSubs<Self::Message>,
    ) -> Subscription<Self::Message> {
        // Since we're wrapping around `E::Message`, we need to do this transformation ourselves
        let mut editor_window_subs = WindowSubs {
            on_frame: match window_subs.on_frame.clone() {
                Some(cb) => {
                    if let Some(Message::EditorMessage(msg)) = cb() {
                        Some(Arc::new(move || { Some(msg.clone()) }))
                    } else { None }
                },
                _ => None
            },
            on_window_will_close: match window_subs.on_window_will_close.clone() {
                Some(cb) => {
                    if let Some(Message::EditorMessage(msg)) = cb() {
                        Some(Arc::new(move || {Some(msg.clone())}))
                    } else { None }
                },
                _ => None
            },
        };

        // turn stream receiver into

        // let parameter_updates = futures::stream::unfold(
        //     self.parameter_updates_receiver.clone(),
        //     |receiver| async move {
        //         // Try to receive a message without blocking
        //         match receiver.try_recv() {
        //             Ok(_) => Some((Message::ParameterUpdate, receiver)),
        //             Err(_) => None,
        //         }
        //     },
        // ).into_(
        //         |msg| futures::future::ready(match msg {
        //             Some()
        //         })
        //     );

        // TODO: does this work correctly?
        let subscription = Subscription::batch(
            self.parameter_updates_receiver
                .clone()
                .iter()
                .map(
                |_| {
                        self.editor
                            .subscription(&mut editor_window_subs)
                            .map(Message::EditorMessage)
                    }
            ),
        );

        if let Some(sub) = editor_window_subs.on_frame {
            if let Some(message) = sub() {
                window_subs.on_frame = Some(Arc::new(move || { Some(Message::EditorMessage(message.clone())) }));
            } else { window_subs.on_frame = None; }
        }
        if let Some(sub) = editor_window_subs.on_window_will_close {
            if let Some(message) = sub() {
                window_subs.on_window_will_close = Some(Arc::new(move || { Some(Message::EditorMessage(message.clone())) }));
            } else { window_subs.on_window_will_close = None; }
        }

        subscription
    }

    #[inline]
    fn view(&self) -> Element<'_, Self::Message> {
        self.editor.view().map(Message::EditorMessage)
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

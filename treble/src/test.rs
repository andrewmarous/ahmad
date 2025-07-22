use crate::editor::ui;

use baseview::WindowScalePolicy;
use crossbeam::atomic::AtomicCell;
use crossbeam::channel;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Editor, GuiContext};
use serde::{Deserialize, Serialize};
// This doesn't need to be re-export but otherwise the compiler complains about
// `hidden_glob_reexports`
pub use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Create an [`Editor`] instance using [iced](https://github.com/iced-rs/iced). The rough idea is
/// that you implement [`IcedEditor`], which is roughly analogous to iced's regular [`Application`]
/// trait except that it receives the [`GuiContext`] alongside its initialization flags so it can
/// update the parameter values. The [`IcedState`] passed to this function contains the GUI's
/// intitial size, and this is kept in sync whenever the GUI gets resized. You can also use this to
/// know if the GUI is open, so you can avoid performing potentially expensive calculations while
/// the GUI is not open. If you want this size to be persisted when restoring a plugin instance,
/// then you can store it in a `#[persist = "key"]` field on your parameters struct.
///
/// See the [module's documentation][self] for an example on how to use this.
pub fn create_iced_editor<E: IcedEditor>(
    iced_state: Arc<IcedState>,
    initialization_flags: E::InitializationFlags,
) -> Option<Box<dyn Editor>> {
    // We need some way to communicate parameter changes to the `IcedEditor` since parameter updates
    // come from outside of the editor's reactive model. This contains only capacity to store only
    // one parameter update, since we're only storing _that_ a parameter update has happened and not
    // which parameter so we'd need to redraw the entire GUI either way.
    let (parameter_updates_sender, parameter_updates_receiver) = channel::bounded(1);

    Some(Box::new(editor::IcedEditorWrapper::<E> {
        iced_state,
        initialization_flags,

        // TODO: We can't get the size of the window when baseview does its own scaling, so if the
        //       host does not set a scale factor on Windows or Linux we should just use a factor of
        //       1. That may make the GUI tiny but it also prevents it from getting cut off.
        #[cfg(target_os = "macos")]
        scaling_factor: AtomicCell::new(None),
        #[cfg(not(target_os = "macos"))]
        scaling_factor: AtomicCell::new(Some(1.0)),

        parameter_updates_sender,
        parameter_updates_receiver: Arc::new(parameter_updates_receiver),
    }))
}

/// A plugin editor using `iced`. This wraps around [`Application`] with the only change being that
/// the usual `new()` function now additionally takes a `Arc<dyn GuiContext>` that the editor can
/// store to interact with the parameters. The editor should have a `Arc<impl Params>` as part
/// of their [`InitializationFlags`][Self::InitializationFlags] so it can read the current parameter
/// values. See [`Application`] for more information.
pub trait IcedEditor: 'static + Send + Sync + Sized {
    /// See [`Application::Executor`]. You'll likely want to use [`crate::executor::Default`].
    type Executor: Executor;
    /// See [`Application::Message`]. You should have one variant containing a [`ParamMessage`].
    type Message: 'static + Clone + Debug + Send;
    /// See [`Application::Flags`].
    type InitializationFlags: 'static + Clone + Send + Sync;

    /// See [`Application::new`]. This also receivs the GUI context in addition to the flags.
    fn new(
        initialization_fags: Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>);

    /// Returns a reference to the GUI context.
    /// [`handle_param_message()`][Self::handle_param_message()] uses this to interact with the
    /// parameters.
    fn context(&self) -> &dyn GuiContext;

    /// See [`Application::update`]. When receiving the variant that contains a
    /// [`widgets::ParamMessage`] you can call
    /// [`handle_param_message()`][Self::handle_param_message()] to handle the parameter update.
    fn update(
        &mut self,
        message: Self::Message,
    ) -> Task<Self::Message>;

    /// See [`Application::subscription`].
    fn subscription(
        &self,
        _window_subs: &mut WindowSubs<Self::Message>,
    ) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// See [`Application::view`].
    fn view(&mut self) -> Element<'_, Self::Message>;

    // /// See [`Application::background_color`].
    // fn background_color(&self) -> Color {
    //     Color::WHITE
    // }

    /// Handle a parameter update using the GUI context.
    fn handle_param_message(&self, message: ParamMessage) {
        // We can't use the fancy ParamSetter here because this needs to be type erased
        let context = self.context();
        match message {
            ParamMessage::BeginSetParameter(p) => unsafe { context.raw_begin_set_parameter(p) },
            ParamMessage::SetParameterNormalized(p, v) => unsafe {
                context.raw_set_parameter_normalized(p, v)
            },
            ParamMessage::EndSetParameter(p) => unsafe { context.raw_end_set_parameter(p) },
        }
    }
}

/// State for an `nih_plug_iced` editor.
#[derive(Debug, Serialize, Deserialize)]
pub struct IcedState {
    /// The window's size in logical pixels before applying `scale_factor`.
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    size: AtomicCell<(u32, u32)>,
    /// Whether the editor's window is currently open.
    #[serde(skip)]
    open: AtomicBool,
}

impl<'a> PersistentField<'a, IcedState> for Arc<IcedState> {
    fn set(&self, new_value: IcedState) {
        self.size.store(new_value.size.load());
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&IcedState) -> R,
    {
        f(self)
    }
}

impl IcedState {
    /// Initialize the GUI's state. This value can be passed to [`create_iced_editor()`]. The window
    /// size is in logical pixels, so before it is multiplied by the DPI scaling factor.
    pub fn from_size(width: u32, height: u32) -> Arc<IcedState> {
        Arc::new(IcedState {
            size: AtomicCell::new((width, height)),
            open: AtomicBool::new(false),
        })
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size.load()
    }

    /// Whether the GUI is currently visible.
    // Called `is_open()` instead of `open()` to avoid the ambiguity.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
}

/// A marker struct to indicate that a parameter update has happened.
pub(crate) struct ParameterUpdate;

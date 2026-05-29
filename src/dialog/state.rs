//! Dialog content and result types.

/// The content of a modal dialog.
///
/// `D` is an application-owned "purpose" payload (often an enum) that the app
/// uses to tell apart which dialog was answered. It is opaque to the library.
///
/// `DialogData` is stored as the focus manager's dialog payload via
/// [`DialogData::show_intent`](crate::dialog::DialogData::show_intent); the
/// focus manager tracks which button is active and navigates between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogData<D = ()> {
    pub title: String,
    pub message: String,
    pub buttons: Vec<String>,
    pub purpose: Option<D>,
    pub is_loading: bool,
}

impl<D> DialogData<D> {
    /// A dialog with a title, message, selectable buttons, and a purpose used
    /// to interpret the selection.
    pub fn new(
        title: impl Into<String>,
        message: impl Into<String>,
        buttons: impl IntoIterator<Item = impl Into<String>>,
        purpose: D,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            buttons: buttons.into_iter().map(Into::into).collect(),
            purpose: Some(purpose),
            is_loading: false,
        }
    }

    /// A buttonless "please wait" dialog. Replace it with [`DialogData::new`]
    /// (via [`show_intent`](DialogData::show_intent)) once the work completes.
    pub fn loading(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            buttons: Vec::new(),
            purpose: None,
            is_loading: true,
        }
    }

    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }
}

/// The outcome of a dialog interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult<D> {
    /// The dialog was closed without choosing a button (e.g. a loading dialog,
    /// or one dismissed with Esc).
    Dismissed,
    /// A button was chosen. `purpose` is the dialog's payload; `index` is which
    /// button (0-based, left to right).
    Selected { purpose: Option<D>, index: usize },
}

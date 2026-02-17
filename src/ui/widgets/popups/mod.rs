use crate::app::App;
use ratatui::Frame;

pub mod audio_info;
pub mod help;
pub mod input;
pub mod tag_editor;
pub mod toast;

pub fn render(f: &mut Frame, app: &mut App) {
    // AUDIO INFO POPUP
    if app.show_audio_info {
        audio_info::render(f, app);
    }

    // TOAST NOTIFICATION
    if let Some(ref _val) = app.toast {
        toast::render(f, app);
    }

    // INPUT POPUP
    if app.input_state.is_some() {
        input::render(f, app);
    }

    // TAG EDITOR POPUP
    if app.tag_edit.is_some() {
        tag_editor::render(f, app);
    }

    // FOOTER / WHICHKEY POPUP
    if app.show_keyhints {
        help::render(f, app);
    }
}

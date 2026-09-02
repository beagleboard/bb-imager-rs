//! Global keyboard shortcuts for the GUI.
//!
//! List navigation and Escape are handled even when the runtime marks an event
//! as captured (macOS often does this while no widget is focused). Tab and
//! Enter defer to widgets when captured so text fields keep working.

use iced::Subscription;
use iced::event::{self, Status};
use iced::keyboard::{self, Key, key::Named};

use crate::message::BBImagerMessage;

pub(crate) fn subscription() -> Subscription<BBImagerMessage> {
    event::listen_with(key_message)
}

fn key_message(
    event: iced::Event,
    status: Status,
    _window: iced::window::Id,
) -> Option<BBImagerMessage> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };

    if (modifiers.control() || modifiers.command())
        && matches!(key, Key::Character(ref c) if c.as_str() == "f")
    {
        return Some(BBImagerMessage::FocusSearch);
    }

    match key {
        Key::Named(Named::ArrowUp) => Some(BBImagerMessage::KeyboardListPrevious),
        Key::Named(Named::ArrowDown) => Some(BBImagerMessage::KeyboardListNext),
        Key::Named(Named::Escape) => Some(BBImagerMessage::KeyboardEscape),
        Key::Named(Named::Enter) if status == Status::Ignored => {
            Some(BBImagerMessage::KeyboardEnter)
        }
        Key::Named(Named::Tab) if status == Status::Ignored => Some(BBImagerMessage::KeyboardTab {
            shift: modifiers.shift(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_press(key: Key, modifiers: keyboard::Modifiers) -> iced::Event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn focus_search_shortcut() {
        let msg = key_message(
            key_press(Key::Character("f".into()), keyboard::Modifiers::COMMAND),
            Status::Captured,
            iced::window::Id::unique(),
        );
        assert!(matches!(msg, Some(BBImagerMessage::FocusSearch)));
    }

    #[test]
    fn arrow_keys_work_when_captured() {
        assert!(matches!(
            key_message(
                key_press(Key::Named(Named::ArrowDown), keyboard::Modifiers::empty()),
                Status::Captured,
                iced::window::Id::unique(),
            ),
            Some(BBImagerMessage::KeyboardListNext)
        ));
    }

    #[test]
    fn enter_ignored_when_captured() {
        assert!(
            key_message(
                key_press(Key::Named(Named::Enter), keyboard::Modifiers::empty()),
                Status::Captured,
                iced::window::Id::unique(),
            )
            .is_none()
        );
    }

    #[test]
    fn enter_when_ignored() {
        assert!(matches!(
            key_message(
                key_press(Key::Named(Named::Enter), keyboard::Modifiers::empty()),
                Status::Ignored,
                iced::window::Id::unique(),
            ),
            Some(BBImagerMessage::KeyboardEnter)
        ));
    }

    #[test]
    fn shift_tab_when_ignored() {
        assert!(matches!(
            key_message(
                key_press(Key::Named(Named::Tab), keyboard::Modifiers::SHIFT),
                Status::Ignored,
                iced::window::Id::unique(),
            ),
            Some(BBImagerMessage::KeyboardTab { shift: true })
        ));
    }

    #[test]
    fn escape_is_delivered_even_when_captured() {
        assert!(matches!(
            key_message(
                key_press(Key::Named(Named::Escape), keyboard::Modifiers::empty()),
                Status::Captured,
                iced::window::Id::unique(),
            ),
            Some(BBImagerMessage::KeyboardEscape)
        ));
    }
}

//! Edit key bindings.

use druid::{KbKey, KeyEvent};

use xi_text_core::{EditOp, Movement};

/// A map from keys to edit commands.
///
/// For now, this is just a stateless map, but it could load
/// preferences or do vi-like bindings.
#[derive(Default)]
pub struct KeyBindings;

impl KeyBindings {
    pub fn map_key(&mut self, k: &KeyEvent) -> Option<EditOp> {
        match &k.key {
            KbKey::Character(c) => {
                // TODO: make this logic more sophisticated
                if !k.mods.ctrl() {
                    Some(EditOp::Insert(c.clone()))
                } else {
                    None
                }
            }
            KbKey::Enter => Some(EditOp::Insert("\n".into())),
            KbKey::Backspace => Some(EditOp::Backspace),
            KbKey::ArrowLeft => Some(EditOp::Move(Movement::Left)),
            KbKey::ArrowRight => Some(EditOp::Move(Movement::Right)),
            KbKey::ArrowUp => Some(EditOp::Move(Movement::Up)),
            KbKey::ArrowDown => Some(EditOp::Move(Movement::Down)),
            _ => None,
        }
    }
}

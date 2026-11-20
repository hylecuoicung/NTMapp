//! Which Enter key went down. winit and egui report the main keyboard's Enter and the numeric
//! keypad's Enter as the same key, but Photoshop tells them apart in type: the main Enter starts a
//! new line, the keypad's commits the text (upstream photocraft#15084). On Windows the UI thread's
//! message hook ([`crate::windows`]) sees each `WM_KEYDOWN` before winit does and notes here
//! whether it was the keypad's Enter (the extended-key flag); the UI takes the notes once a frame
//! and pairs them, in order, with that frame's Enter presses.

use std::sync::{Mutex, PoisonError};

/// `WM_KEYDOWN` (winuser.h).
pub const WM_KEYDOWN: u32 = 0x0100;
/// `VK_RETURN` (winuser.h): either Enter key.
pub const VK_RETURN: usize = 0x0D;
/// Bit 24 of a key message's `lParam` (`KF_EXTENDED`): set for the numeric keypad's Enter.
pub const EXTENDED_KEY: isize = 1 << 24;
/// Notes nobody takes (a build without the UI reading them) stop growing here; the oldest go.
const MAX_PENDING: usize = 32;

static ENTER_KEYS: Mutex<Vec<bool>> = Mutex::new(Vec::new());

/// Note a key message the UI thread took off its queue. An Enter key going down, auto-repeat
/// included (winit reports every repeat as a press), adds whether it was the keypad's Enter.
pub fn note_key_message(message: u32, w_param: usize, l_param: isize) {
    if message != WM_KEYDOWN || w_param != VK_RETURN {
        return;
    }
    let mut keys = ENTER_KEYS.lock().unwrap_or_else(PoisonError::into_inner);
    if keys.len() >= MAX_PENDING {
        keys.remove(0);
    }
    keys.push(l_param & EXTENDED_KEY != 0);
}

/// The Enter presses noted since the last call, oldest first: `true` for the numeric keypad's.
pub fn take_enter_keys() -> Vec<bool> {
    std::mem::take(&mut *ENTER_KEYS.lock().unwrap_or_else(PoisonError::into_inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test owns the shared notes, so parallel tests cannot interleave with it.
    #[test]
    fn enter_presses_are_noted_in_order_with_their_key() {
        let _ = take_enter_keys();
        // Scan code 0x1C with a repeat count of 1: the main Enter, then the keypad's (extended).
        let main = 0x001C_0001;
        note_key_message(WM_KEYDOWN, VK_RETURN, main);
        note_key_message(WM_KEYDOWN, VK_RETURN, main | EXTENDED_KEY);
        // Other keys, and Enter going up, are not presses of Enter.
        note_key_message(WM_KEYDOWN, 0x41, main);
        note_key_message(0x0101, VK_RETURN, main | EXTENDED_KEY);
        assert_eq!(take_enter_keys(), [false, true]);
        assert_eq!(take_enter_keys(), [] as [bool; 0], "taking empties the notes");
        for i in 0..MAX_PENDING + 5 {
            note_key_message(WM_KEYDOWN, VK_RETURN, if i == MAX_PENDING + 4 { EXTENDED_KEY } else { 0 });
        }
        let kept = take_enter_keys();
        assert_eq!(kept.len(), MAX_PENDING, "notes nobody takes stop growing");
        assert_eq!(kept.last(), Some(&true), "the newest are kept");
    }
}

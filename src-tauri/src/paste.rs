use anyhow::Result;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Copies text to the clipboard, triggers Cmd+V / Ctrl+V into the focused input field,
/// and restores the user's previous clipboard contents.
pub fn paste_into_active_field(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();

    clipboard.set_text(text.to_string())?;
    thread::sleep(Duration::from_millis(45));

    let mut enigo = Enigo::new(&Settings::default())?;

    #[cfg(target_os = "macos")]
    {
        enigo.key(Key::Meta, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Meta, Direction::Release)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        enigo.key(Key::Control, Direction::Press)?;
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Control, Direction::Release)?;
    }

    thread::sleep(Duration::from_millis(140));
    if let Some(prev) = previous {
        let _ = clipboard.set_text(prev);
    }
    Ok(())
}

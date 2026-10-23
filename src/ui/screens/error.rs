//! Screens for expected failures: a broken config, an entry that cannot boot.
//! Unlike the crash screen they may allocate, and the caller decides which
//! ways on to offer.

use alloc::{format, string::String, vec, vec::Vec};

use effie::protocols::InputKey;

use crate::{
    error::Error,
    logfile,
    ui::{
        font::Font,
        input,
        report::{self, Report},
        theme,
    },
};

/// A way on from an error screen.
#[derive(Clone, Copy)]
pub enum Action {
    BackToMenu,
    Reboot,
    FirmwareSetup,
}

impl Action {
    fn hint(self) -> (&'static str, &'static str) {
        match self {
            Action::BackToMenu => ("ENTER", "back to menu"),
            Action::Reboot => ("R", "reboot"),
            Action::FirmwareSetup => ("F2", "firmware setup"),
        }
    }

    fn matches(self, key: &InputKey) -> bool {
        match self {
            Action::BackToMenu => {
                key.scan_code == InputKey::SCAN_NULL && key.unicode_char == u16::from(b'\r')
            }
            Action::Reboot => {
                u8::try_from(key.unicode_char).is_ok_and(|c| c.eq_ignore_ascii_case(&b'r'))
            }
            Action::FirmwareSetup => key.scan_code == InputKey::SCAN_F2,
        }
    }
}

/// Logs `error`, saves the log, and shows the error until a key picks one of
/// `actions`. After `timeout` seconds the answer is [`Action::Reboot`].
pub fn show(
    font: &Font,
    title: &str,
    error: &Error,
    actions: &[Action],
    timeout: Option<u32>,
) -> Action {
    log::error!("{error}");
    // A copy, so the history keeps recording: saving the file logs too.
    let history = effie_log::copy_history();
    let saved = logfile::save(&history, &[]).is_some();
    drop(history);

    let body = format!("{error}");
    let reboot_note: String;
    let mut notes = vec![logfile::note(saved)];
    if let Some(seconds) = timeout {
        reboot_note = format!("bread reboots in {seconds} seconds");
        notes.push(&reboot_note);
    }
    let keys: Vec<_> = actions.iter().map(|action| action.hint()).collect();

    let st = effie::system_table();
    let boot_services = st.boot_services();
    let report = Report {
        palette: &theme::MENU,
        font,
        title,
        body: &body,
        notes: &notes,
        keys: &keys,
    };
    report::show(st, boot_services, &report);

    let key = input::wait_for_fresh(st, boot_services, timeout, |key| {
        actions.iter().any(|action| action.matches(key))
    });
    match key {
        Some(key) => actions
            .iter()
            .copied()
            .find(|action| action.matches(&key))
            .unwrap_or(Action::Reboot),
        None if timeout.is_some() => Action::Reboot,
        // No keyboard and nothing to wait for. Rebooting would most likely
        // come back to this same screen, so stay on it, like the crash screen.
        None => loop {
            core::hint::spin_loop();
        },
    }
}

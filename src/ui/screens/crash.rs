//! The crash screen, shown by the panic handler. Allocation-free, like
//! everything it draws with.

use effie::tables::{BootServices, ResetType, SystemTable};

use crate::{
    logfile,
    ui::{
        font::EMBEDDED,
        input,
        report::{self, Report},
        theme,
    },
};

/// Saves the log, shows `body` and reboots on any key. Returns if there is
/// no keyboard.
pub fn show(st: &SystemTable, boot_services: &BootServices, body: &str) {
    // Taken first, so the log ends with the panic.
    let (older, newer) = effie_log::take_history();
    let saved = logfile::save(older, newer).is_some();
    let report = Report {
        palette: &theme::CRASH,
        font: &EMBEDDED,
        title: "bread crashed",
        body,
        notes: &[
            logfile::note(saved),
            "Please report this at https://github.com/tailwags/bread",
        ],
        keys: &[("Any key", "reboot")],
    };
    report::show(st, boot_services, &report);
    if input::wait_for_fresh(st, boot_services, None, |_| true).is_some() {
        st.runtime_services().reset_system(ResetType::Cold);
    }
}

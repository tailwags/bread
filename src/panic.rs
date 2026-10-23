//! The panic handler. A panic means bread itself is broken; expected failures
//! are [`Error`](crate::error::Error)s and get their own screens instead. It
//! reports the panic on every channel that still works, then waits for a key
//! and reboots.
//!
//! The panic is logged like any other error, so it reaches the serial port
//! and ends the log history, which is saved to `\EFI\bread\bread.log`. It is
//! also shown on the screen (graphics, or the text console as a fallback).
//! Every step is best-effort and allocation-free, since the heap may be what
//! failed. A panic inside the handler halts.

use core::{fmt::Write, panic::PanicInfo};

use effie_log::Buf;

use crate::ui::screens::crash;

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    if !effie_log::report_panic(info) {
        halt();
    }

    // Formatted apart, so a long message cannot push out the location.
    let mut message = Buf::<512>::new();
    let _ = write!(message, "{}", info.message());

    let mut body = Buf::<704>::new();
    let _ = match info.location() {
        Some(at) => write!(
            body,
            "{}\n\nat {}:{}:{}",
            message.as_str(),
            at.file(),
            at.line(),
            at.column()
        ),
        None => write!(body, "{}", message.as_str()),
    };

    // The screen, the file system and the keyboard all need boot services.
    // SAFETY: effie stores a valid system table pointer before calling `main`.
    let system_table = effie::system_table_raw().map(|table| unsafe { table.as_ref() });

    if let Some(st) = system_table
        && let Some(boot_services) = st.boot_services_opt()
    {
        crash::show(st, boot_services, body.as_str());
    }

    halt()
}

fn halt() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

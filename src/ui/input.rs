//! Reading keys.

use effie::{
    Status,
    protocols::{InputKey, SimpleTextInput},
    tables::{BootServices, SystemTable},
};

use crate::error::{Context, Result};

/// How long the keyboard must be quiet before a key counts, so a key still
/// held from the menu (auto-repeating) cannot dismiss a screen before it is
/// seen. Longer than the usual 500 ms typematic delay.
const QUIET_MS: usize = 600;
const POLL_MS: usize = 10;

/// Waits for a key, however long it takes.
pub fn wait(con_in: &mut SimpleTextInput, boot_services: &BootServices) -> Result<InputKey> {
    let events = [con_in.wait_for_key_event()];
    loop {
        boot_services
            .wait_for_event(&events)
            .context("waiting for a key")?;
        if let Ok(key) = con_in.read_key_stroke() {
            return Ok(key);
        }
    }
}

/// Returns a key pressed within a second, or `None`.
pub fn within_a_second(
    con_in: &mut SimpleTextInput,
    boot_services: &BootServices,
) -> Option<InputKey> {
    for _ in 0..100 {
        if let Ok(key) = con_in.read_key_stroke() {
            return Some(key);
        }
        let _ = boot_services.stall(10_000);
    }
    None
}

/// Discards pending keys, then waits for a key that `accept` takes. Returns
/// `None` once `timeout` seconds have passed, or straight away if there is no
/// keyboard and no timeout. Allocation-free, for the panic handler.
pub fn wait_for_fresh(
    st: &SystemTable,
    boot_services: &BootServices,
    timeout: Option<u32>,
    accept: impl Fn(&InputKey) -> bool,
) -> Option<InputKey> {
    let mut con_in = st.con_in().ok();
    if let Some(con_in) = &mut con_in {
        let _ = con_in.reset(false);
    }

    let mut left = timeout.map(|seconds| seconds as usize * 1000);
    let mut quiet = 0;
    loop {
        if let Some(left) = &mut left {
            *left = left.checked_sub(POLL_MS)?;
        }
        match con_in.as_mut().map(|con_in| con_in.read_key_stroke()) {
            Some(Ok(key)) if quiet >= QUIET_MS => {
                if accept(&key) {
                    return Some(key);
                }
            }
            Some(Ok(_)) => quiet = 0,
            Some(Err(Status::NOT_READY)) => quiet += POLL_MS,
            // No keyboard: wait out the timeout, if there is one.
            _ if timeout.is_some() => con_in = None,
            _ => return None,
        }
        let _ = boot_services.stall(POLL_MS * 1000);
    }
}

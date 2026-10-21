//! Rebooting, and rebooting into the firmware's own setup UI through the
//! `OsIndications` variable (UEFI specification §8.5.4).

use effie::tables::{ResetType, VariableAttributes};
use effie::{Guid, Status, WStr, guid, w};

use crate::error::{Context, Result};

/// `EFI_GLOBAL_VARIABLE`, the vendor of the `OsIndications` variables.
const GLOBAL_VARIABLE: Guid = guid!("8be4df61-93ca-11d2-aa0d-00e098032b8c");
/// `EFI_OS_INDICATIONS_BOOT_TO_FW_UI`
const BOOT_TO_FW_UI: u64 = 1 << 0;

/// Whether the firmware can stop in its setup UI on the next boot.
pub fn setup_supported() -> bool {
    read_u64(w!("OsIndicationsSupported")).is_ok_and(|bits| bits & BOOT_TO_FW_UI != 0)
}

/// Asks the firmware to show its setup UI on the next boot, then reboots.
/// Returns only on failure.
pub fn reboot_to_setup() -> Result<!> {
    let name = w!("OsIndications");
    let bits = match read_u64(name) {
        Ok(bits) => bits,
        Err(Status::NOT_FOUND) => 0,
        Err(status) => return Err(status).context("reading OsIndications"),
    };
    let attributes = VariableAttributes::NON_VOLATILE
        | VariableAttributes::BOOTSERVICE_ACCESS
        | VariableAttributes::RUNTIME_ACCESS;
    let bits = (bits | BOOT_TO_FW_UI).to_le_bytes();
    effie::system_table()
        .runtime_services()
        .set_variable(name, &GLOBAL_VARIABLE, attributes, &bits)
        .context("setting OsIndications")?;
    log::info!("rebooting into the firmware setup");
    reboot()
}

pub fn reboot() -> ! {
    effie::system_table()
        .runtime_services()
        .reset_system(ResetType::Cold)
}

fn read_u64(name: &WStr) -> effie::Result<u64> {
    let mut bytes = [0; 8];
    // SAFETY: boot services are still running.
    let (_, len) = unsafe {
        effie::system_table()
            .runtime_services()
            .get_variable(name, &GLOBAL_VARIABLE, &mut bytes)
    }?;
    if len != bytes.len() {
        return Err(Status::BAD_BUFFER_SIZE);
    }
    Ok(u64::from_le_bytes(bytes))
}

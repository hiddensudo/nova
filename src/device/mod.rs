pub mod chatmix;
pub mod eq;
mod hidraw;
mod protocol;
pub mod settings;
mod transaction;
mod usbfs;

use std::{io, sync::Mutex, time::Duration};

use hidraw::{Hidraw, HidrawPort};
use protocol::{READ_TIMEOUT, transact};
use usbfs::UsbPort;

pub use protocol::{Operation, Sidetone};

// Report ID prefix for Linux hidraw plus the 64-byte USB payload.
const REPORT_SIZE: usize = 65;
static USB_TRANSACTION: Mutex<()> = Mutex::new(());

pub fn detect() -> Result<String, String> {
    let (path, transport, availability) = match HidrawPort::find()? {
        Some(port) => {
            let availability = port.open().map(|_| ());
            (port.path, "hidraw", availability)
        }
        None => {
            let port = UsbPort::find()?;
            let availability = port.claim().map(|_| ());
            (port.path, "usbfs", availability)
        }
    };
    let availability = availability.err().unwrap_or_else(|| "доступний".into());
    Ok(format!(
        "Arctis Nova 7 Gen 2 · 1038:227e · USB HID 3 ({transport}) · {} · {availability}",
        path.display()
    ))
}

#[cfg(feature = "gui")]
pub fn dongle_present() -> bool {
    UsbPort::present()
}

pub fn sidetone(operation: Operation) -> Result<Sidetone, String> {
    with_device(|port| transact(port, operation, READ_TIMEOUT))
}

fn with_device<T>(
    operation: impl FnOnce(&mut dyn HidReports) -> Result<T, String>,
) -> Result<T, String> {
    let _serial = USB_TRANSACTION.lock().map_err(|error| error.to_string())?;
    let _process_lock = transaction::acquire(true)?;
    open_device(operation)
}

fn try_with_device<T>(
    operation: impl FnOnce(&mut dyn HidReports) -> Result<T, String>,
) -> Result<Option<T>, String> {
    let Ok(_serial) = USB_TRANSACTION.try_lock() else {
        return Ok(None);
    };
    let Some(_process_lock) = transaction::acquire(false)? else {
        return Ok(None);
    };
    open_device(operation).map(Some)
}

fn open_device<T>(
    operation: impl FnOnce(&mut dyn HidReports) -> Result<T, String>,
) -> Result<T, String> {
    match HidrawPort::find()? {
        Some(port) => {
            let file = port.open()?;
            file.try_lock().map_err(|error| {
                format!("Донгл зайнятий або блокування недоступне. Спробуй ще раз. ({error})")
            })?;
            operation(&mut Hidraw(file))
        }
        None => operation(&mut UsbPort::find()?.claim()?),
    }
}

trait HidReports {
    fn write_report(&mut self, report: &[u8; REPORT_SIZE]) -> io::Result<()>;
    fn read_report(
        &mut self,
        reply: &mut [u8; REPORT_SIZE],
        timeout: Duration,
    ) -> io::Result<usize>;
}

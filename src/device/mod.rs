pub mod chatmix;
pub mod eq;
mod hidraw;
mod model;
pub mod probe;
mod protocol;
pub mod settings;
mod transaction;
mod usbfs;

use std::{io, sync::Mutex, time::Duration};

use hidraw::{Hidraw, HidrawPort};
use model::Model;
use protocol::{READ_TIMEOUT, transact};
use usbfs::UsbPort;

pub use protocol::{Operation, Sidetone};

// Report ID prefix for Linux hidraw plus the 64-byte USB payload.
const REPORT_SIZE: usize = 65;
static USB_TRANSACTION: Mutex<()> = Mutex::new(());

pub fn detect() -> Result<String, String> {
    let model = Model::NOVA_7_GEN2;
    let (path, transport, availability) = match HidrawPort::find(model)? {
        Some(port) => {
            let availability = port.open().map(|_| ());
            (port.path, "hidraw", availability)
        }
        None => {
            let port = UsbPort::find(model)?;
            let availability = port.claim().map(|_| ());
            (port.path, "usbfs", availability)
        }
    };
    let availability = availability.err().unwrap_or_else(|| "available".into());
    Ok(format!(
        "{} · {:04x}:{:04x} · USB HID 3 ({transport}) · {} · {availability}",
        model.name(),
        Model::VENDOR,
        model.product(),
        path.display()
    ))
}

#[cfg(feature = "gui")]
pub fn dongle_present() -> bool {
    UsbPort::present(Model::NOVA_7_GEN2)
}

pub fn sidetone(operation: Operation) -> Result<Sidetone, String> {
    with_device(|port| transact(port, operation, READ_TIMEOUT))
}

fn with_device<T>(
    operation: impl FnOnce(&mut dyn HidReports) -> Result<T, String>,
) -> Result<T, String> {
    with_model(Model::NOVA_7_GEN2, operation)
}

fn with_model<T>(
    model: Model,
    operation: impl FnOnce(&mut dyn HidReports) -> Result<T, String>,
) -> Result<T, String> {
    let _serial = USB_TRANSACTION.lock().map_err(|error| error.to_string())?;
    let _process_lock = transaction::acquire(true)?;
    open_device(model, operation)
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
    open_device(Model::NOVA_7_GEN2, operation).map(Some)
}

fn open_device<T>(
    model: Model,
    operation: impl FnOnce(&mut dyn HidReports) -> Result<T, String>,
) -> Result<T, String> {
    match HidrawPort::find(model)? {
        Some(port) => {
            let file = port.open()?;
            file.try_lock().map_err(|error| {
                format!("Dongle busy or lock unavailable. Try again. ({error})")
            })?;
            operation(&mut Hidraw(file))
        }
        None => operation(&mut UsbPort::find(model)?.claim()?),
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

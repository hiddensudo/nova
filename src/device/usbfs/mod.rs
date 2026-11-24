mod reports;

use std::{
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Read},
    os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

use reports::UsbConnection;

use super::Model;

const CONTROL_INTERFACE: u16 = 3;

pub(super) struct UsbPort {
    pub path: PathBuf,
    model: Model,
}

impl UsbPort {
    pub fn probe_models() -> Result<Vec<Model>, String> {
        let devices = fs::read_dir("/sys/bus/usb/devices")
            .map_err(|error| format!("Cannot access USB devices in sysfs ({error})."))?;
        Ok(devices
            .flatten()
            .filter_map(|entry| device_ids(&entry.path()))
            .filter_map(|(vendor, product)| Model::for_product(vendor, product))
            .collect())
    }

    pub fn find(model: Model) -> Result<Self, String> {
        let devices = fs::read_dir("/sys/bus/usb/devices")
            .map_err(|error| format!("Cannot access USB devices in sysfs ({error})."))?;
        let mut ports = devices
            .flatten()
            .filter_map(|entry| find_port(&entry.path(), model));
        match (ports.next(), ports.next()) {
            (Some(port), None) => Ok(port),
            (Some(_), Some(_)) => Err("Connect only one matching headset dongle.".into()),
            _ => Err("No matching headset USB control interface found.".into()),
        }
    }

    #[cfg(feature = "gui")]
    pub fn present(model: Model) -> bool {
        fs::read_dir("/sys/bus/usb/devices").is_ok_and(|devices| {
            devices
                .flatten()
                .any(|entry| matches_model(&entry.path(), model))
        })
    }

    fn open_device(&self) -> Result<File, String> {
        let mut file = OpenOptions::new().read(true).write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&self.path)
            .map_err(|error| match error.kind() {
                ErrorKind::PermissionDenied => format!("Cannot access {}. Check the Nova Control udev rule.", self.path.display()),
                _ => format!("Could not open {} ({error}). Run Nova in a local Linux session with the dongle connected.", self.path.display()),
            })?;
        let mut descriptor = [0; 18];
        let is_device = file
            .metadata()
            .is_ok_and(|meta| meta.file_type().is_char_device());
        // usbfs read begins with the cached USB device descriptor. Verify VID/PID
        // again after opening in case Linux reused the address during a hotplug.
        if !is_device
            || file.read_exact(&mut descriptor).is_err()
            || descriptor[..2] != [18, 1]
            || !self.model.matches(
                u16::from_le_bytes([descriptor[8], descriptor[9]]),
                u16::from_le_bytes([descriptor[10], descriptor[11]]),
            )
        {
            return Err("The dongle USB address changed. Try again.".into());
        }
        Ok(file)
    }

    pub fn claim(&self) -> Result<UsbConnection, String> {
        let file = self.open_device()?;
        let interface = u32::from(CONTROL_INTERFACE);
        // Claim only the dedicated HID interface. Closing the file releases it.
        // If usbhid or another process owns it, the kernel rejects the claim.
        let result = unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                libc::_IOR::<u32>(b'U'.into(), 15),
                &interface,
            )
        };
        if result < 0 {
            let error = io::Error::last_os_error();
            return Err(format!(
                "HID control interface 3 is unavailable ({error}). Close any other headset manager and try again."
            ));
        }
        Ok(UsbConnection(file))
    }
}

fn find_port(device: &Path, model: Model) -> Option<UsbPort> {
    if !matches_model(device, model) {
        return None;
    }
    let name = device.file_name()?.to_str()?;
    let configuration = read(device, "bConfigurationValue")?;
    let interface = device.join(format!("{name}:{configuration}.{CONTROL_INTERFACE}"));
    if read(&interface, "bInterfaceClass")? != "03"
        || read(&interface, "bAlternateSetting")?.parse::<u8>().ok()? != 0
        || read(&interface.join("ep_84"), "type")? != "Interrupt"
    {
        return None;
    }
    let bus: u16 = read(device, "busnum")?.parse().ok()?;
    let address: u8 = read(device, "devnum")?.parse().ok()?;
    Some(UsbPort {
        model,
        path: PathBuf::from(format!("/dev/bus/usb/{bus:03}/{address:03}")),
    })
}

fn read(path: &Path, attribute: &str) -> Option<String> {
    Some(
        fs::read_to_string(path.join(attribute))
            .ok()?
            .trim()
            .to_owned(),
    )
}

fn matches_model(device: &Path, model: Model) -> bool {
    device_ids(device).is_some_and(|(vendor, product)| model.matches(vendor, product))
}

fn device_ids(device: &Path) -> Option<(u16, u16)> {
    let id =
        |attribute| read(device, attribute).and_then(|value| u16::from_str_radix(&value, 16).ok());
    Some((id("idVendor")?, id("idProduct")?))
}

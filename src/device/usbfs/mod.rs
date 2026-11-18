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

const CONTROL_INTERFACE: u16 = 3;

pub(super) struct UsbPort {
    pub path: PathBuf,
}

impl UsbPort {
    pub fn find() -> Result<Self, String> {
        let devices = fs::read_dir("/sys/bus/usb/devices")
            .map_err(|error| format!("Немає доступу до USB у sysfs ({error})."))?;
        let mut ports = devices
            .flatten()
            .filter_map(|entry| find_port(&entry.path()));
        match (ports.next(), ports.next()) {
            (Some(port), None) => Ok(port),
            (Some(_), Some(_)) => Err("Залиш підключеним один донгл Nova 7 Gen 2.".into()),
            _ => {
                Err("Керувальний USB-інтерфейс донгла Nova 7 Gen 2 (1038:227e) не знайдено.".into())
            }
        }
    }

    #[cfg(feature = "gui")]
    pub fn present() -> bool {
        fs::read_dir("/sys/bus/usb/devices")
            .is_ok_and(|devices| devices.flatten().any(|entry| is_nova_dongle(&entry.path())))
    }

    fn open_device(&self) -> Result<File, String> {
        let mut file = OpenOptions::new().read(true).write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&self.path)
            .map_err(|error| match error.kind() {
                ErrorKind::PermissionDenied => format!("Немає доступу до {}. Перевір правило udev для 1038:227e.", self.path.display()),
                _ => format!("Не вдалося відкрити {} ({error}). Запусти з локального термінала Linux із підключеним донглом.", self.path.display()),
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
            || descriptor[8..12] != [0x38, 0x10, 0x7e, 0x22]
        {
            return Err("USB-адреса донгла змінилася. Повтори спробу.".into());
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
                "Керувальний HID-інтерфейс 3 недоступний ({error}). Закрий інший менеджер навушників, якщо він відкритий, і повтори спробу."
            ));
        }
        Ok(UsbConnection(file))
    }
}

fn find_port(device: &Path) -> Option<UsbPort> {
    if !is_nova_dongle(device) {
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

fn is_nova_dongle(device: &Path) -> bool {
    read(device, "idVendor").as_deref() == Some("1038")
        && read(device, "idProduct").as_deref() == Some("227e")
}

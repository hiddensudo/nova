use std::{
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Read, Write},
    os::{
        fd::{AsFd, AsRawFd},
        unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::Duration,
};

use super::{HidReports, REPORT_SIZE};

const USB_ID: &str = "HID_ID=0003:00001038:0000227E";

pub(super) struct HidrawPort {
    pub(super) path: PathBuf,
    device_number: u64,
}

impl HidrawPort {
    pub(super) fn find() -> Result<Option<Self>, String> {
        let devices = match fs::read_dir("/sys/class/hidraw") {
            Ok(devices) => devices,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("Немає доступу до Linux hidraw у sysfs ({error}).")),
        };
        let mut ports = devices
            .flatten()
            .filter_map(|device| inspect_port(&device.path()));
        let port = ports.next();
        if ports.next().is_some() {
            return Err(
                "Виявлено кілька керувальних HID. Залиш підключеним один донгл Nova 7 Gen 2."
                    .into(),
            );
        }
        Ok(port)
    }

    pub(super) fn open(&self) -> Result<File, String> {
        let file = OpenOptions::new().read(true).write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(&self.path)
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => format!("Вузол {} недоступний. Запусти з локального термінала Linux із підключеним донглом.", self.path.display()),
                ErrorKind::PermissionDenied => format!("Немає доступу до {}. Перевір правило udev для 1038:227e.", self.path.display()),
                _ => format!("Не вдалося відкрити {} ({error}).", self.path.display()),
            })?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("Перевірка вузла HID не вдалася ({error})."))?;
        if !metadata.file_type().is_char_device() || metadata.rdev() != self.device_number {
            return Err("Вузол HID змінився. Повтори підключення.".into());
        }
        Ok(file)
    }
}

fn inspect_port(entry: &Path) -> Option<HidrawPort> {
    let device = entry.join("device");
    let uevent = fs::read_to_string(device.join("uevent")).ok()?;
    if !uevent.lines().any(|line| line.eq_ignore_ascii_case(USB_ID)) {
        return None;
    }
    let interface = fs::read_to_string(device.join("../bInterfaceNumber")).ok()?;
    let interface = u8::from_str_radix(interface.trim(), 16).ok()?;
    let descriptor = fs::read(device.join("report_descriptor")).ok()?;
    if interface != 3 || !is_control_descriptor(&descriptor) {
        return None;
    }
    let number = fs::read_to_string(entry.join("dev")).ok()?;
    let (major, minor) = number.trim().split_once(':')?;
    let device_number = libc::makedev(major.parse().ok()?, minor.parse().ok()?);
    Some(HidrawPort {
        path: Path::new("/dev").join(entry.file_name()?),
        device_number,
    })
}

fn is_control_descriptor(descriptor: &[u8]) -> bool {
    if !matches!(descriptor, [0x06, 0xc0, 0xff, ..]) {
        return false;
    }
    let mut has_output = false;
    let mut items = descriptor;
    while let Some((&tag, rest)) = items.split_first() {
        // Known control collections use short items and unnumbered reports.
        let size = [0, 1, 2, 4][usize::from(tag & 3)];
        if tag == 0xfe || tag & 0xfc == 0x84 || rest.len() < size {
            return false;
        }
        has_output |= tag & 0xfc == 0x90;
        items = &rest[size..];
    }
    has_output
}

pub(super) struct Hidraw(pub(super) File);

impl HidReports for Hidraw {
    fn write_report(&mut self, report: &[u8; REPORT_SIZE]) -> io::Result<()> {
        match self.0.write(report)? {
            REPORT_SIZE => Ok(()),
            count => Err(io::Error::other(format!(
                "неповний запис {count}/{REPORT_SIZE} байтів"
            ))),
        }
    }

    fn read_report(
        &mut self,
        reply: &mut [u8; REPORT_SIZE],
        timeout: Duration,
    ) -> io::Result<usize> {
        let mut descriptor = libc::pollfd {
            fd: self.0.as_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // poll uses a live descriptor and a single initialized pollfd.
        let ready = unsafe {
            libc::poll(
                &mut descriptor,
                1,
                timeout.as_millis().max(1) as libc::c_int,
            )
        };
        match ready {
            0 => Err(ErrorKind::TimedOut.into()),
            -1 => Err(io::Error::last_os_error()),
            _ if descriptor.revents & libc::POLLIN != 0 => self.0.read(reply),
            _ => Err(ErrorKind::NotConnected.into()),
        }
    }
}

use std::{ffi::c_void, fs::File, io, os::fd::AsRawFd, time::Duration};

use super::CONTROL_INTERFACE;
use crate::device::{HidReports, REPORT_SIZE};

const INPUT_ENDPOINT: u32 = 0x84;

pub(in crate::device) struct UsbConnection(pub(super) File);

// Layouts from linux/usbdevice_fs.h.
#[repr(C)]
struct ControlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    data: *mut c_void,
}

#[repr(C)]
struct EndpointTransfer {
    endpoint: u32,
    length: u32,
    timeout: u32,
    data: *mut c_void,
}

impl HidReports for UsbConnection {
    fn write_report(&mut self, report: &[u8; REPORT_SIZE]) -> io::Result<()> {
        // SET_REPORT carries the HID report ID in wValue, omitting the prefix on the wire.
        let mut payload = [0; REPORT_SIZE - 1];
        payload.copy_from_slice(&report[1..]);
        let mut transfer = ControlTransfer {
            request_type: 0x21, // host-to-device, HID class, interface recipient
            request: 0x09,      // SET_REPORT
            value: 0x0200,      // output report, no report ID
            index: CONTROL_INTERFACE,
            length: payload.len() as u16,
            timeout: 750,
            data: payload.as_mut_ptr().cast(),
        };
        // Both the C-layout transfer and its buffer live for the synchronous ioctl.
        let result = unsafe {
            libc::ioctl(
                self.0.as_raw_fd(),
                libc::_IOWR::<ControlTransfer>(b'U'.into(), 0),
                &mut transfer,
            )
        };
        match result {
            -1 => Err(io::Error::last_os_error()),
            count if count as usize == payload.len() => Ok(()),
            count => Err(io::Error::other(format!(
                "неповний запис USB ({count}/{} байтів)",
                payload.len()
            ))),
        }
    }

    fn read_report(
        &mut self,
        reply: &mut [u8; REPORT_SIZE],
        timeout: Duration,
    ) -> io::Result<usize> {
        let mut transfer = EndpointTransfer {
            endpoint: INPUT_ENDPOINT,
            length: (REPORT_SIZE - 1) as u32,
            timeout: timeout.as_millis().max(1) as u32,
            data: reply.as_mut_ptr().cast(),
        };
        // USBDEVFS_BULK also accepts interrupt endpoints. The call is synchronous.
        let result = unsafe {
            libc::ioctl(
                self.0.as_raw_fd(),
                libc::_IOWR::<EndpointTransfer>(b'U'.into(), 2),
                &mut transfer,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(result as usize)
    }
}

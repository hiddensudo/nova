use std::io;

use super::*;
use std::collections::VecDeque;

#[derive(Default)]
struct TestPort {
    sent: Vec<[u8; REPORT_SIZE]>,
    received: VecDeque<Vec<u8>>,
}

impl HidReports for TestPort {
    fn write_report(&mut self, report: &[u8; REPORT_SIZE]) -> io::Result<()> {
        self.sent.push(*report);
        Ok(())
    }

    fn read_report(
        &mut self,
        buffer: &mut [u8; REPORT_SIZE],
        _timeout: Duration,
    ) -> io::Result<usize> {
        let reply = self.received.pop_front().ok_or(ErrorKind::TimedOut)?;
        buffer[..reply.len()].copy_from_slice(&reply);
        Ok(reply.len())
    }
}

#[test]
fn read_skips_status_and_rejects_invalid_settings() {
    let mut port = TestPort {
        received: [vec![0xb0], vec![SETTINGS, 7, 2, 0]].into(),
        ..Default::default()
    };
    assert_eq!(
        transact(&mut port, Operation::Read, READ_TIMEOUT),
        Ok(Sidetone::Medium)
    );
    assert!(decode_settings(&[SETTINGS, 7, 2]).is_err());
    assert!(decode_settings(&[SETTINGS, 7, 4, 0]).is_err());
}

#[test]
fn set_verifies_level_without_writing_persistent_settings() {
    let mut port = TestPort {
        received: [vec![SETTINGS, 7, 0, 0], vec![SETTINGS, 7, 3, 0]].into(),
        ..Default::default()
    };
    assert_eq!(
        transact(&mut port, Operation::Set(Sidetone::High), READ_TIMEOUT),
        Ok(Sidetone::High)
    );
    let mut read = [0; REPORT_SIZE];
    read[1] = SETTINGS;
    let mut set = [0; REPORT_SIZE];
    set[1..3].copy_from_slice(&[SET_SIDETONE, 3]);
    assert_eq!(port.sent, [read, set, read]);
}

#[test]
fn explicit_save_verifies_active_level_before_and_after_the_save_frame() {
    let mut port = TestPort {
        received: [vec![SETTINGS, 7, 2, 0], vec![SETTINGS, 7, 2, 0]].into(),
        ..Default::default()
    };
    assert_eq!(
        transact(&mut port, Operation::Save, READ_TIMEOUT),
        Ok(Sidetone::Medium)
    );
    assert_eq!(
        port.sent.iter().map(|frame| frame[1]).collect::<Vec<_>>(),
        [SETTINGS, SAVE, SETTINGS]
    );
    assert_eq!(port.sent[1].len(), 65);
    assert!(port.sent[1][2..].iter().all(|&byte| byte == 0));
}

use super::*;

#[test]
fn decodes_captured_nova_7_gen_2_settings() {
    let actual = decode(&[0xb0, 3, 20, 3], &[0x20, 7, 3, 0], &[0xa0, 30, 2, 1]).unwrap();
    assert_eq!(actual.battery, Some(20));
    assert_eq!(actual.charging, ChargeState::Discharging);
    assert_eq!(actual.mic_volume, 7);
    assert_eq!(actual.sleep_minutes, 30);
    assert_eq!(actual.mute_led, 2);
}

#[test]
fn validates_settings_frames_and_rejects_invalid_writes() {
    assert!(decode(&[0xb0, 3, 80], &[AUDIO, 7], &[UX, 30, 2]).is_err());
    assert_eq!(
        decode(&[0xb0, 2, 99, 1], &[AUDIO, 7], &[UX, 30, 2])
            .unwrap()
            .battery,
        None
    );
    assert_eq!(Change::MicVolume(4).command(), Ok([0x37, 4]));
    assert_eq!(Change::SleepMinutes(45).command(), Ok([0xa3, 45]));
    assert_eq!(Change::MuteLed(3).command(), Ok([0xae, 3]));
    assert!(Change::MicVolume(8).command().is_err());
    assert!(Change::MuteLed(4).command().is_err());
}

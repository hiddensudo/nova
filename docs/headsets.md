# Headset support

| Headset | USB dongle | Nova Control status |
| --- | --- | --- |
| Arctis Nova 7 Gen 2 | `1038:227e` | Hardware verified. Battery, Sidetone, EQ, mic gain, mute LED, auto-off and ChatMix. |
| Arctis Nova 7X Gen 2 | `1038:229e` | Experimental CLI status probe. Connect the 2.4 GHz dongle in USB mode and run `nova-control probe 229e`. It reads wireless status without changing headset settings. |

For a 7X test report, run the probe with the headset on and again with the wheel centered and at each end. Include the USB ID and the reported values. The first GUI milestone is status, battery and ChatMix on real hardware. The settings and EQ paths require separate verification per device.

Planned order after 7X Gen 2:

1. Original Nova 7 and 7X. Verify the charging flags and the discrete battery format on earlier firmware.
2. Nova 7P. Verify battery and EQ separately. The physical wheel controls Sidetone rather than ChatMix.
3. Nova 5 and 5X, followed by Nova 3 Wireless and Nova Pro Wireless.

[HeadsetControl device table and capabilities](https://github.com/Sapd/HeadsetControl/blob/master/lib/devices/steelseries_arctis_nova_7.hpp) · [Upstream Nova 7X Gen 2 ChatMix test](https://github.com/rdamron/rust-arctis-chatmix)

## Develop without the headset

Nova’s HID transport trait can replay synthetic responses in tests. The 7X probe test checks the exact status request and simulated center and endpoint reports. This is appropriate for parsers, ranges, timeouts and UI behavior. Treat synthetic fixtures as assumptions until an owner captures equivalent reports from a real dongle. A software mock cannot confirm where the headset routes Sidetone, whether a write persists, or which firmware accepts a command.

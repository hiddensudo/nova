# Headset support

| Headset | USB dongle | Nova Control status |
| --- | --- | --- |
| Arctis Nova 7 Gen 2 | `1038:227e` | Hardware verified. Battery, Sidetone, EQ, mic gain, mute LED, auto-off and ChatMix. |
| Arctis Nova 7X Gen 2 | `1038:229e` | Experimental CLI status probe. Connect the 2.4 GHz dongle in USB mode and run `nova-control probe 229e`. It reads wireless status without changing headset settings. |
| Arctis Nova 7 Gen 1 | `1038:2202`, `1038:22a1` | Experimental CLI status probe. Tested with simulated HID replies. |
| Arctis Nova 7X Gen 1 | `1038:2206`, `1038:22a4`, `1038:22a5` | Experimental CLI status probe. Tested with simulated HID replies. |

The Gen 1 probes and automatic detection require Nova Control 0.3.10 or newer. With one supported dongle connected, run `nova-control probe`. If several are connected, pass its USB product ID, for example `nova-control probe 2206`. For a test report, capture the center and both ends of ChatMix with the headset on, a reading with the headset off, and a reading while charging. Include the USB ID and reported values. The GUI and controls for these headsets require separate hardware verification.

[Submit a headset compatibility report](https://github.com/hiddensudo/nova/issues/new?template=headset.yml)

Gen 1 dongles `2202`, `2206` and `22a4` report five battery levels, from 0 to 4. Nova presents these as 0%, 25%, 50%, 75% and 100%. The firmware variants `22a1` and `22a5` report a percentage directly. [HeadsetControl device table and battery decoder](https://github.com/Sapd/HeadsetControl/blob/master/lib/devices/steelseries_arctis_nova_7.hpp)

1. Confirm the experimental 7X Gen 2, Nova 7 and 7X Gen 1 probes with their physical dongles.
2. Nova 7P. Verify battery and EQ separately. The physical wheel controls Sidetone.
3. Nova 5 and 5X, followed by Nova 3 Wireless and Nova Pro Wireless.

[HeadsetControl device table and capabilities](https://github.com/Sapd/HeadsetControl/blob/master/lib/devices/steelseries_arctis_nova_7.hpp) · [Upstream Nova 7X Gen 2 ChatMix test](https://github.com/rdamron/rust-arctis-chatmix)

## Develop without the headset

Nova’s HID transport trait can replay synthetic responses in tests. The probe tests verify the status request, ChatMix center and endpoints, the Gen 1 battery encodings and an offline headset. Treat synthetic fixtures as assumptions until an owner captures equivalent reports from a real dongle. A software mock cannot confirm where the headset routes Sidetone, whether a write persists, or which firmware accepts a command.

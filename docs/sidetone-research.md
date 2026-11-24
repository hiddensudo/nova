# Hardware Sidetone on Nova 7 Gen 2

The 2.4 GHz dongle tested is `1038:227e`. Nova can set and verify the four Sidetone levels. Read-only queries also return a separate microphone EQ profile. A listening check with Arctis Companion did not reveal a Sidetone change when switching the Mic EQ preset. Other firmware revisions and stronger EQ changes remain untested.

SteelSeries documents Sidetone as a feed of microphone input into the headset. It also documents Sonar ClearCast AI as processing in a PC audio channel. Its support explanation of raw monitoring is consistent with our observations. The Nova 7 Gen 2 product and Companion documentation list microphone EQ and Sidetone as separate controls. We found no documented control that inserts a noise gate or noise reduction into this model’s internal Sidetone path.

A controlled next test would play the same tone and keyboard sample into the mic, record both the USB microphone stream and the sound in one earcup at a fixed Sidetone level, and compare them before and after a clearly audible Mic EQ change. Use a known reversible command and restore the starting curve. The result can establish whether Mic EQ reaches the monitor path. It cannot establish that the firmware offers a separate noise gate.

Sources: [SteelSeries Sidetone support](https://support.steelseries.com/hc/en-us/articles/360051003711-What-is-Sidetone/), [raw monitor explanation](https://support.steelseries.com/hc/en-us/articles/45990582122509-Fix-White-Noise-or-Static-When-Using-Active-Noise-Cancellation), [Nova 7 Gen 2 Companion guide](https://steelseries.com/es-co/blog/how-to-use-the-arctis-companion-app-with-arctis-nova-7-gen-2), [Sonar’s per-channel processing](https://steelseries.com/en-th/blog/share-your-sonar-config-and-win-1072).

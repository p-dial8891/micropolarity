# Core 1

A standalone binary that is to be programmed to core 1 of the RP2350. It is responsible for the following functions:
1. Audio decoding and playback
2. Wifi connectivity

## Audio decoding and playback
Audio data is read via interprocessor communications and decoded in software using a double buffer.

## Wifi connectivity
The software listens on port 1234, after connecting to the network using hardcoded credentials, for the name of the track to play. A tcp connection is closed once playback of the track is complete and, then starts over listening for the next track name to play.

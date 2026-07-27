# Vesper Virtual Audio

Personal-use Windows WaveRT virtual audio driver for Vesper DSP.

Planned endpoints:

- `Vesper DSP Input` — select this as the playback device in Tidal or another player.
- `Vesper DSP Capture` — select this as the audio source in Vesper DSP.

The device contract is stereo 32-bit PCM at 44.1, 48, 88.2, 96, 176.4, 192,
352.8, or 384 kHz. The driver source is
based on Microsoft's SimpleAudioSample under MS-PL; see `NOTICES.md`.

The current build is intentionally test-signing only. Do not enable Windows test
signing or change Secure Boot settings automatically; those are user-controlled
boot-security changes and require a reboot.

The package build produces `VesperVirtualAudio.sys`, `VesperVirtualAudio.inf`,
and the signed `vespervirtualaudio.cat` catalog.

Build the personal test artifact with:

```powershell
& .\Build-VesperVirtualAudio.ps1
```

After the package is built, run `Install-VesperVirtualAudio-Test.ps1` from an
elevated PowerShell session. It trusts the generated personal test certificate,
enables Windows test signing, and schedules the root-device installation for
the required next reboot.

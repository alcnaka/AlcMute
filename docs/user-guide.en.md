# AlcMute User Guide

AlcMute is a Windows 10 / 11 app that lets you toggle your VRChat microphone using a shortcut or the system tray, even while another app is active.

## Installation and updates

[Download the latest release](https://github.com/alcnaka/AlcMute/releases/latest). For most users, the Installer edition (Setup.exe) is recommended.

| | Installer | Portable |
| --- | --- | --- |
| Getting started | Run Setup.exe | Extract the ZIP and run alcmute.exe |
| Start at Windows sign-in | Supported | Not supported |
| Updating | Update from the app | Replace the EXE using a new ZIP |

If AlcMute is already running, choose Exit from its tray menu before installing. Your existing settings are kept.

After installation, the Installer edition runs in the system tray. Open Settings from the Start menu or the tray menu.

To uninstall, use Windows Settings > Apps. Your saved settings are kept.

## Basic operation

### Getting started

1. Enable **OSC** and **Toggle Voice** in VRChat.
2. Start AlcMute.
3. Press **Ctrl + Alt + M** to toggle mute.

The tray icon is **green when the microphone is active, red when muted, and gray with a question mark when the state is unknown**.

Right-click the tray icon to toggle mute, open Settings, or exit. If the icon is hidden, check the tray overflow menu on the right side of the taskbar.

Closing Settings with Close or the X button leaves AlcMute running in the tray. To stop it, choose Exit from the tray menu or Quit app in Settings.

### Changing the shortcut

1. In Settings, click Change keys.
2. Press your key combination, then release all keys.
3. Click Save. Press Esc to cancel recording.

To distinguish left and right Ctrl, Alt, and other modifier keys, enable Distinguish left and right modifiers before recording. Changing this checkbox does not alter your existing shortcut. Modifier-only combinations and combinations such as K + L are also supported.

Shortcuts work while Settings is open. They pause only during key recording. If another app responds to the same keys, choose a different combination.

You can also record the two standard mouse side buttons as Mouse4 or Mouse5, alone or combined with keys such as Ctrl. Their normal actions in other apps, such as Back and Forward, are not blocked. If your mouse software remaps a button to a keyboard key, AlcMute may recognize that key instead.

![Settings example in Japanese. The shortcut shown is an example](screenshots/v0.3.0/08-settings-dark-connected.jpg)

## Changing settings

Click Save after changing settings. Save is enabled only when there are unsaved changes.

- **Tray double-click**: Toggle mute or open Settings.
- **Start at Windows sign-in**: Available in the Installer edition.
- **Show Settings when launched manually**: Turn this off to start in the tray.
- **Theme**: Choose System, Light, or Dark. System follows your Windows setting.
- **Language**: Choose System, Japanese, or English, then save. System uses Japanese on Japanese Windows and English on other Windows display languages.

The circular arrow beside a setting restores its default value. Reset all settings asks for confirmation before deleting saved settings. It also disables sign-in startup for the Installer edition.

You normally do not need to change the OSC addresses. The defaults are 127.0.0.1:9000 for sending and 127.0.0.1:9001 for receiving.

## Updates

Click Check for updates in Settings or the tray menu. Follow the on-screen instructions if a newer version is available.

To disable checks at startup, clear Check for updates on startup and click Save.

For the Portable edition, download the new ZIP, exit AlcMute, then replace the EXE.

## Troubleshooting

### Blocked by security software

The current app and installer are not code-signed. If you see a warning or block, check that the file came from the [official GitHub Releases](https://github.com/alcnaka/AlcMute/releases), then note the full message and affected filename. A missing signature alone does not prove that a file is safe or malicious.

- **SmartScreen**: Checks downloaded apps against reputation and other information. Files without an established reputation may trigger a warning. This is separate from Smart App Control. [Microsoft's explanation](https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/)
- **Smart App Control (Windows 11)**: May block an app when it cannot establish safety and no valid signature is available. There is no per-app allow option. Blocking the current installer's startup-check EXE can produce the message that AlcMute's running state could not be checked. The Portable edition may also be blocked. [Microsoft FAQ](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions)
- **Defender and other antivirus software**: Check protection history or quarantine records for the detection name, affected file, and time. Do not assume it is a false positive; [report these details to the developer](https://github.com/alcnaka/AlcMute/issues).

Disabling protection or adding broad exclusions is not part of the normal installation instructions. Contact your administrator on a managed PC. Conditions for re-enabling protection depend on Windows updates, so also read the explanation shown on your device.

For a detailed installer log, open PowerShell in the download folder and run the following, replacing the Setup filename with the one you downloaded. The log is saved to your desktop.

```powershell
.\AlcMute-0.3.2-Setup.exe /LOG="$env:USERPROFILE\Desktop\AlcMute-setup.log"
```

Before sharing a log, check for usernames and personal folder paths, and redact them as needed.

### Mute does not toggle

Check that OSC and Toggle Voice are enabled in VRChat. If only the shortcut fails, try toggling mute from the tray menu, then change the shortcut and save.

A gray question mark means AlcMute cannot read the mute state. It does not mean you are muted. Check the microphone state in VRChat too.

### An update fails

Exit AlcMute and run the [latest Setup.exe](https://github.com/alcnaka/AlcMute/releases/latest) manually. Your settings are kept. This also works around os error 31 when updating from v0.3.0 or v0.3.1.

### AlcMute will not start or save settings

Check the error message. If AlcMute offers to restore a damaged settings file, you can recover it from the backup.

When asking for help, include your AlcMute version, what you did, and the error message. If a log is needed, error.log is in this folder:

```text
%LOCALAPPDATA%\Alcnaka\AlcMute
```

This folder also contains your settings. You do not need to delete your settings file to ask for help.

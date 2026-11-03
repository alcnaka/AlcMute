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

First, check that you downloaded the file from the [official GitHub Releases](https://github.com/alcnaka/AlcMute/releases). Then follow the steps for the message you see.

#### Edge says the file is not commonly downloaded

If you have checked the source and want to continue downloading:

1. Open Downloads at the top right of Edge, or press Ctrl + J.
2. Hover over the file, open its three-dot menu, and select Keep.
3. When Edge asks you to make sure you trust the file, open the arrow next to Delete at the bottom right and select Keep anyway.
4. Once the download finishes, open the saved Setup.exe.

Some versions of Edge show Show more followed by Keep anyway in step 3. These steps apply to the uncommon-download warning. If the message says a virus was detected, follow the antivirus instructions below instead.

#### Windows says it protected your PC when you start the file

If you have checked the source and want to continue, select More info, then Run anyway. If Run anyway is unavailable, or your work or school manages the PC, stop and ask your administrator.

#### Smart App Control blocks the file

The Keep anyway and Run anyway steps above cannot clear this block. Note the full warning and affected filename, then [contact the developer](https://github.com/alcnaka/AlcMute/issues). If Setup says it cannot check whether AlcMute is running, also check Windows notifications or protection history for a block. Switching to the Portable edition may not resolve it.

Smart App Control has no per-app allow option. Leave protection enabled and pause installation if you cannot proceed. [Microsoft FAQ](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions)

#### Antivirus software detects or quarantines the file

Open Windows Security > Virus & threat protection > Protection history, or the history in your antivirus app. Send the detection name, affected filename, and time to the developer. Leave the file quarantined and do not add an exclusion while it is being investigated.

The current downloads are not code-signed, so the publisher may appear as Unknown. If you are unsure how to proceed, contact the developer with the warning message.

### Mute does not toggle

Check that OSC and Toggle Voice are enabled in VRChat. If only the shortcut fails, try toggling mute from the tray menu, then change the shortcut and save.

A gray question mark means AlcMute cannot read the mute state. It does not mean you are muted. Check the microphone state in VRChat too.

### An update fails

**Updating from v0.3.0 or v0.3.1 requires a manual installation. Thank you for your patience.**

1. Choose Exit from the tray menu to quit AlcMute.
2. Download the [latest Setup.exe](https://github.com/alcnaka/AlcMute/releases/latest).
3. Run Setup.exe and follow the installation instructions. Your existing settings are kept.

If an update fails on another version, you can use the same steps to install manually.

### AlcMute will not start or save settings

Check the error message. If AlcMute offers to restore a damaged settings file, you can recover it from the backup.

When asking for help, include your AlcMute version, what you did, and the error message. If a log is needed, error.log is in this folder:

```text
%LOCALAPPDATA%\Alcnaka\AlcMute
```

This folder also contains your settings. You do not need to delete your settings file to ask for help.

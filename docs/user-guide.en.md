# AlcMute User Guide

AlcMute is a Windows 10 / 11 app that lets you toggle your VRChat microphone using a shortcut or the system tray, even while another app is active.

## Installation

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

Check these settings to connect to VRChat:

1. **Enable OSC**: Open the VRChat Action Menu (press `R` on desktop), then turn on Options > OSC > Enabled.
2. **Set the microphone to toggle mode**: In the VRChat Main Menu, open Settings > Audio & Voice > Microphone and set Microphone Behaviour to Toggle. This corresponds to the former Toggle Voice setting.
3. **Start AlcMute**: The Installer edition is available in the Start menu.
4. **Press `Ctrl + Alt + M`** to toggle the microphone mute.

OSC carries messages between AlcMute and VRChat. Toggle changes the microphone's mute state each time it is activated.

#### Tray icon legend

| Icon | VRChat microphone status |
| --- | --- |
| ![](../assets/mic-unmuted.svg) | **Green · Microphone active** |
| ![](../assets/mic-muted.svg) | **Red · Muted** |
| ![](../assets/mic-unknown.svg) | **Gray question mark · Unknown**. Check the status in VRChat. |

Right-click the tray icon to toggle mute, open Settings, or exit. If the icon is hidden, check the tray overflow menu on the right side of the taskbar.

Closing Settings with Close or the X button leaves AlcMute running in the tray. To stop it, choose Exit from the tray menu or Quit app in Settings.

### Changing the shortcut

1. In Settings, click Change keys.
2. Press your key combination, then release all keys.
3. Click Save. Press Esc to cancel recording.

#### Distinguish left and right modifiers

To register left Ctrl and right Ctrl as different keys, enable **Distinguish left and right modifiers before recording**.

Changing this option **does not change the registered shortcut**. Record the shortcut again to apply the selection.

#### Supported inputs

- **Keyboard**: Combinations such as `Ctrl + Alt + M`, modifier-only combinations, and combinations such as `K + L`.
- **Mouse**: Record the side buttons (`Mouse4` and `Mouse5`) alone or with keys such as Ctrl.

Use Change keys to record mouse side buttons. Their actions in other apps, such as Back and Forward, are not blocked.

If your mouse software remaps a button to a keyboard key, AlcMute may recognize that key instead.

![Settings example in Japanese. The shortcut shown is an example](screenshots/v0.3.0/08-settings-dark-connected.jpg)

## Changing settings

Click Save after changing settings. Save is enabled when there are unsaved changes.

| Setting | Description |
| --- | --- |
| Tray double-click | Toggle mute or open Settings |
| Start at Windows sign-in | Installer edition only |
| Show Settings when launched manually | Turn off to start in the tray |
| Theme | System, Light, or Dark |
| Language | System, Japanese, or English |

The System theme follows Windows. The System language uses Japanese when the Windows display language is Japanese and English otherwise.

### OSC addresses

You normally do not need to change these defaults.

| Direction | Address |
| --- | --- |
| AlcMute to VRChat (send) | `127.0.0.1:9000` |
| VRChat to AlcMute (receive) | `127.0.0.1:9001` |

## Updates

Click Check for updates in Settings or the tray menu. Follow the on-screen instructions if a newer version is available.

To disable checks at startup, clear Check for updates on startup and click Save.

For the Portable edition, download the new ZIP, exit AlcMute, then replace the EXE.

## Troubleshooting

### Blocked by security software

First, check that you downloaded the file from the [official GitHub Releases](https://github.com/alcnaka/AlcMute/releases). **The steps depend on the warning displayed.**

| Warning | See |
| --- | --- |
| The file is not commonly downloaded (Edge) | [Edge download warning](#edge-download-warning) |
| Windows protected your PC | [Windows protection screen](#windows-protection-screen) |
| Blocked by Smart App Control | [Smart App Control](#smart-app-control) |
| A virus was detected | [Antivirus detection and quarantine](#antivirus-detection-and-quarantine) |

#### Edge download warning

If you have checked the source and want to continue downloading:

1. Open Downloads at the top right of Edge, or press Ctrl + J.
2. Hover over the file, open its three-dot menu, and select Keep.
3. When Edge asks you to make sure you trust the file, open the arrow next to Delete at the bottom right and select Keep anyway.
4. Once the download finishes, open the saved Setup.exe.

Some versions of Edge show Show more followed by Keep anyway in step 3. These steps apply to the uncommon-download warning. If the message says a virus was detected, see [Antivirus detection and quarantine](#antivirus-detection-and-quarantine).

#### Windows protection screen

If you have checked the source and want to continue, select More info, then Run anyway. If Run anyway is unavailable, or your work or school manages the PC, stop and ask your administrator.

#### Smart App Control

When Smart App Control is enabled, **AlcMute cannot be allowed individually**. See the [Microsoft FAQ](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions).

#### Antivirus detection and quarantine

Open Windows Security > Virus & threat protection > Protection history, or the history in your antivirus app. Send the detection name, affected filename, and time to the developer. Leave the file quarantined and do not add an exclusion while it is being investigated.

### Mute does not toggle

Check the following in order:

1. **VRChat settings**: Check that OSC is enabled and Microphone Behaviour is set to Toggle. See [Getting started](#getting-started).
2. **Tray controls**: Right-click the AlcMute tray icon and toggle mute from the menu.
3. **Shortcut**: If the tray control works, change the shortcut and click Save.

A gray question mark means **AlcMute cannot read the mute state**. It does not mean you are muted. Check the microphone state in VRChat too.

### An update fails

If the in-app update fails, try the manual installation steps below.

#### Updating from v0.3.0 or v0.3.1

**Updating from either of these versions requires a manual installation.**

#### Manual installation

1. Choose Exit from the tray menu to quit AlcMute.
2. Download the [latest Setup.exe](https://github.com/alcnaka/AlcMute/releases/latest).
3. Run Setup.exe and follow the installation instructions. Your existing settings are kept.

### AlcMute will not start or save settings

1. **Check the error**: Note the message shown on screen.
2. **Check the recovery prompt**: If AlcMute offers to restore a damaged settings file, follow the prompt to recover it from the backup.
3. **Ask for help**: Include your AlcMute version, what you did, and the error message in [GitHub Issues](https://github.com/alcnaka/AlcMute/issues).

If a log is needed, `error.log` is located at:

```text
%LOCALAPPDATA%\Alcnaka\AlcMute\error.log
```

The same folder also contains your settings. You do not need to delete your settings file to ask for help.

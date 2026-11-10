# Install Wick

Short version: [README → Install](../README.md#install). Downloads: [GitHub Releases](https://github.com/Abulkosim/wick/releases/latest).

Wick is free and open source. Builds are not signed with an Apple Developer ID or a Windows publisher certificate, so macOS and Windows warn on first launch. Updates are signed separately and verified in the app. winget and Linux package repositories (apt, dnf, Flathub) are not available yet.

| Platform | File |
| --- | --- |
| macOS 12+ · Apple Silicon and Intel | `Wick_<version>_universal.dmg` |
| Windows 10/11 x64 (from v0.2.0) | `Wick_<version>_x64-setup.exe` (recommended) or `Wick_<version>_x64_en-US.msi` |
| Linux x86_64 (from v0.7.0), glibc 2.35+: Ubuntu 22.04, Debian 12, Fedora 36 and newer | `Wick_<version>_amd64.AppImage` (any distribution), `Wick_<version>_amd64.deb` or `Wick-<version>-1.x86_64.rpm` |
| Linux arm64 (from v0.8.0), same distributions | `Wick_<version>_aarch64.AppImage`, `Wick_<version>_arm64.deb` or `Wick-<version>-1.aarch64.rpm` |

## macOS

These steps match macOS 15 Sequoia and macOS 26 Tahoe. Control-click → **Open** no longer skips the warning on these versions.

1. Download `Wick_<version>_universal.dmg`.
2. Open the DMG and drag **Wick** to **Applications**.
3. Open Wick from Applications.
4. macOS shows **“Wick” Not Opened**, saying Apple could not verify it is free of malware. Click **Done**. Do not click Move to Trash.
5. Open **System Settings → Privacy & Security**.
6. Scroll to **Security**. Next to **“Wick” was blocked to protect your Mac**, click **Open Anyway**.
7. Confirm with **Open** and enter your password or use Touch ID.

macOS remembers this choice; later launches open normally. Managed Macs may not allow exceptions. See [Apple: Safely open apps on your Mac](https://support.apple.com/en-us/102445).

On macOS 12–14, Control-click Wick in Applications → **Open** also works.

Fallback only if **Open Anyway** does not appear:

```sh
xattr -d com.apple.quarantine /Applications/Wick.app
```

This removes the download flag macOS uses to trigger the check. Use it only for a file you downloaded from this repository's releases and [verified](#verify-a-download).

Wick asks for no permissions. It has no Dock icon or app window; look for the candle in the menu bar. If the menu bar is full, the icon may be hidden behind the notch or by a menu bar manager.

### Homebrew

```sh
brew install --cask abulkosim/tap/wick
```

The cask installs the same DMG from GitHub Releases and links the `wick` command. The first launch still needs **Open Anyway** (steps 3–7 above); `wick status` and the other commands work after that. Wick updates itself, so `brew upgrade` leaves it alone. `brew uninstall --cask wick` removes it; add `--zap` to also delete settings and history.

## Windows

1. Download `Wick_<version>_x64-setup.exe` (or the `.msi`).
2. Run it. If **Windows protected your PC** appears, click **More info → Run anyway**.
3. The installer adds Wick for the current user. If WebView2 is missing, it [downloads Microsoft's setup program](https://v2.tauri.app/distribute/windows-installer/#downloaded-bootstrapper), so first installation may need internet access.
4. Wick appears in the system tray as a candle. Windows may put new icons in the overflow menu (**^**); drag it onto the taskbar to keep it visible.

A managed device may hide **Run anyway**. See [Microsoft Defender SmartScreen](https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/).

**Smart App Control.** On Windows 11 with Smart App Control on, unsigned apps are blocked with no Run anyway option, and Microsoft offers [no per-app exception](https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003).

**What differs on Windows.** Wick keeps the PC awake with a standard power request (`powercfg /requests` lists it as Wick). It can't list other apps' requests without administrator rights, has no lid warning, and triggers are commands (`ffmpeg`, `claude`), power and monitors rather than apps.

## Linux

Pick one. The commands below are for x86_64; on arm64 the files end in `_aarch64.AppImage`, `_arm64.deb` and `.aarch64.rpm`.

- **AppImage** (any distribution, updates itself):

  ```sh
  chmod +x Wick_<version>_amd64.AppImage
  ./Wick_<version>_amd64.AppImage
  ```

  Keep it somewhere permanent, such as `~/Applications`, before turning on **Launch at login**: the login entry points at the file. If it won't start with a FUSE error, install `libfuse2` (Ubuntu 22.04: `sudo apt install libfuse2`; 24.04: `sudo apt install libfuse2t64`).
- **Debian, Ubuntu, Mint, Pop!_OS:** `sudo apt install ./Wick_<version>_amd64.deb`
- **Fedora, openSUSE:** `sudo dnf install ./Wick-<version>-1.x86_64.rpm`

Open **Wick** from your applications. The candle sits in the panel's tray: left-click opens Wick, right-click shows the menu.

**The tray.** KDE Plasma, Xfce, Cinnamon and Ubuntu's GNOME show it out of the box. Plain GNOME (Fedora, Debian) has no tray: install the [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/) extension, or use Wick without a tray: it then opens as a normal window, and closing that window quits Wick and ends its session.

**How Wick keeps it awake.** Through your desktop's own inhibitors (the XDG portal on GNOME and KDE; PowerManagement and the screensaver on Xfce) and a systemd-logind lock, so `systemd-inhibit --list` shows `Wick` while it's on. Without a desktop that answers, logind also blocks suspend; `systemctl suspend -i` still suspends. Closing the lid still suspends unless your desktop is set to ignore it.

**What differs on Linux.**

- Triggers are commands (`ffmpeg`, `claude`), network, CPU, power, displays and schedules, not apps.
- "Who else" lists logind inhibitors and what GNOME or KDE hold for apps.
- On Wayland, apps can't register global shortcuts or place their own windows. Bind `wick toggle` to a key in your desktop's keyboard settings; the popover opens where your desktop puts it.
- `wick` is on your `PATH` with the `.deb` and `.rpm` (`wick status`, `wick start 45`); with the AppImage, call the file.

**Updates.** The AppImage replaces itself. The `.deb` and `.rpm` install updates through `pkexec`, which asks for your password; you can also update with your package manager by installing the new file.

## Verify a download

Each release includes `SHA256SUMS.txt`. Compare your file's checksum with it:

```sh
shasum -a 256 Wick_<version>_universal.dmg
```

```powershell
Get-FileHash Wick_<version>_x64-setup.exe -Algorithm SHA256
```

```sh
sha256sum Wick_<version>_amd64.AppImage
```

Releases also carry GitHub build attestations and an SPDX SBOM (`wick-<version>.sbom.spdx.json`). With the [GitHub CLI](https://cli.github.com), check that a file was built by this repository's release workflow:

```sh
gh attestation verify Wick_<version>_universal.dmg --repo Abulkosim/wick
```

## Updates

Wick checks GitHub Releases for a new version once a day. Turn this off with **Check for updates daily** in **Settings → Updates**, or check by hand there. Installing is always your choice; Wick downloads, verifies and restarts only when you click install. A dot on the Settings gear means an update is waiting.

Each update is signed with the project's update key, and Wick verifies the signature and version before installing. An altered or mismatched download is rejected and nothing is installed. Update signatures do not replace Apple's publisher signing, so a fresh manual install still shows the warnings above.

Update checks send nothing about you. Wick has no telemetry. See [PRIVACY.md](../PRIVACY.md).

## Uninstall

First turn off **Launch at login** in Wick's Settings, then right-click the candle and choose **Quit Wick**.

**macOS.** Delete `Wick.app` from Applications (or `brew uninstall --cask --zap wick`). To remove settings, triggers and history, delete:

```
~/Library/Application Support/com.abulkosim.wick/
```

If Launch at login was left on, also delete `~/Library/LaunchAgents/Wick.plist`.

**Windows.** Open **Settings → Apps → Installed apps**, choose **Wick → Uninstall**. To remove settings, triggers and history, delete `%APPDATA%\com.abulkosim.wick\`.

**Linux.** `sudo apt remove wick` or `sudo dnf remove wick`, or delete the AppImage. To remove settings, triggers and history, delete `~/.local/share/com.abulkosim.wick/`. If Launch at login was left on, also delete `~/.config/autostart/Wick.desktop`; an AppImage also leaves `~/.local/share/applications/wick-handler.desktop` for `wick://` links.

Quitting releases Wick's power assertion or inhibitors at once; Wick never changes `pmset`, logind settings or other system settings, so there is nothing to restore.

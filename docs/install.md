# Install Wick

Short version: [README → Install](../README.md#install). Downloads: [GitHub Releases](https://github.com/Abulkosim/wick/releases/latest).

Wick is free and open source. Builds are not signed with an Apple Developer ID or a Windows publisher certificate, so macOS and Windows warn on first launch. Updates are signed separately and verified in the app. winget is not available yet.

| Platform | File |
| --- | --- |
| macOS 12+ · Apple Silicon and Intel | `Wick_<version>_universal.dmg` |
| Windows 10/11 x64 (from v0.2.0) | `Wick_<version>_x64-setup.exe` (recommended) or `Wick_<version>_x64_en-US.msi` |

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

## Verify a download

Each release includes `SHA256SUMS.txt`. Compare your file's checksum with it:

```sh
shasum -a 256 Wick_<version>_universal.dmg
```

```powershell
Get-FileHash Wick_<version>_x64-setup.exe -Algorithm SHA256
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

Quitting releases Wick's power assertion at once; Wick never changes `pmset` or other system settings, so there is nothing to restore.

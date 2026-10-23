# Install Wick

Short version: [README → Install](../README.md#install). Downloads: [GitHub Releases](https://github.com/Abulkosim/wick/releases/latest).

Wick is free and open source. Builds are not signed with an Apple Developer ID, so macOS warns on first launch. Updates are signed separately and verified in the app. Homebrew is not available yet.

| Platform | File |
| --- | --- |
| macOS 12+ · Apple Silicon and Intel | `Wick_<version>_universal.dmg` |

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

## Verify a download

Each release includes `SHA256SUMS.txt`. Compare your file's checksum with it:

```sh
shasum -a 256 Wick_<version>_universal.dmg
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

Delete `Wick.app` from Applications. To remove settings and triggers, delete:

```
~/Library/Application Support/com.abulkosim.wick/
```

If Launch at login was left on, also delete `~/Library/LaunchAgents/Wick.plist`.

Quitting releases Wick's power assertion at once; Wick never changes `pmset` or other system settings, so there is nothing to restore.

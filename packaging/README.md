# Packaging

Manifests for package managers. All of them install the files from GitHub Releases, checked against the release's `SHA256SUMS.txt`.

| Channel | Files | Status |
| --- | --- | --- |
| Homebrew (macOS) | `homebrew/Casks/wick.rb` | Live: `abulkosim/tap/wick`, bumped by `homebrew.yml` |
| Scoop (Windows) | `../bucket/wick.json` | Live once on `main`: the bucket is this repository |
| winget (Windows) | `winget/Abulkosim.Wick/<version>/` | Ready to submit 👤 |
| AUR (Arch Linux) | `aur/PKGBUILD`, `aur/.SRCINFO` | Ready to submit 👤 |
| Flathub (Linux) | `flatpak/` | Ready to submit, with limits below 👤 |

## Bump

After a release is published, one command points every channel at it:

```sh
node packaging/bump.mjs vX.Y.Z          # reads SHA256SUMS.txt from the release
node packaging/bump.mjs vX.Y.Z SHA256SUMS.txt
```

It rewrites the cask, the Scoop manifest, the winget folder (renamed to the new version), PKGBUILD and `.SRCINFO`, the Flatpak sources and the metainfo release list, taking the date from `changelogs/vX.Y.Z.md`. It writes nothing if an asset is missing. `node --test packaging/*.test.mjs` checks that every channel names the same version and that `.SRCINFO` matches PKGBUILD.

Commit the result. Scoop users get it from `main`; the other channels need the steps below.

## Updates inside Wick

Scoop, the AUR and Flatpak update Wick themselves, so each install writes a `package-manager` file naming itself (`Scoop`, `the AUR`, `Flatpak`): next to `wick.exe` on Windows, at `/usr/lib/wick/` or `/app/lib/wick/` on Linux. From v0.9.0, Wick then skips its own update checks and Settings → Updates says "Updates come from Scoop." winget and Homebrew install the regular installer, so Wick's own updater stays on there.

## Scoop

```powershell
scoop bucket add wick https://github.com/Abulkosim/wick
scoop install wick/wick
```

Scoop unpacks the MSI (`PFiles\Wick\wick.exe`) without running it, so nothing is written to the registry and no Start menu entry is made beyond Scoop's own shortcut. `checkver` and `autoupdate` follow GitHub's latest release, so `scoop update wick` works even before `bump.mjs` runs.

👤 Before the first announcement, on Windows: run the two commands above, open Wick from the Start menu, start and stop a session, and check Settings → Updates. Then `scoop uninstall wick`.

## winget

`InstallerType: nullsoft`, user scope, `/S`, the same NSIS setup the in-app updater installs. `ManifestVersion` is 1.12.0, like Margin's; the files validate against the 1.12.0 schemas from `microsoft/winget-cli`.

👤 Submit:

1. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) to `Abulkosim`.
2. On Windows: `winget validate --manifest packaging\winget\Abulkosim.Wick\X.Y.Z`, then `winget install --manifest packaging\winget\Abulkosim.Wick\X.Y.Z` (needs `winget settings --enable LocalManifestFiles` from an admin shell once).
3. Submit with `wingetcreate submit --token <PAT> packaging\winget\Abulkosim.Wick\X.Y.Z`, or copy the folder to `manifests/a/Abulkosim/Wick/X.Y.Z/` in the fork and open a pull request.
4. If Defender or SmartScreen flags the installer during review, submit it at the [Microsoft Security Intelligence portal](https://www.microsoft.com/wdsi/filesubmission).
5. Later versions: `wingetcreate update Abulkosim.Wick --version X.Y.Z --urls <setup.exe URL> --submit`, or the same copy and pull request with the bumped folder.

## AUR

`wick-bin` repackages the release `.deb` for `x86_64` and `aarch64`. It depends on `webkit2gtk-4.1` and `gtk3`; the tray uses StatusNotifierItem, so no appindicator library.

Checked here in Debian's `makepkg`: `makepkg --printsrcinfo` matches `.SRCINFO`, the sources pass their checksums, and the package holds `usr/bin/wick`, the desktop entry, icons, the license and `usr/lib/wick/package-manager`. `namcap` only runs on Arch.

👤 Submit:

1. Create an account at [aur.archlinux.org](https://aur.archlinux.org) and add an SSH public key under My Account.
2. On Arch (or an `archlinux` container), as a normal user:

   ```sh
   git clone ssh://aur@aur.archlinux.org/wick-bin.git
   cp packaging/aur/PKGBUILD packaging/aur/.SRCINFO wick-bin/
   cd wick-bin
   makepkg --printsrcinfo | diff - .SRCINFO
   makepkg -si
   namcap PKGBUILD && namcap wick-bin-*.pkg.tar.zst
   git add PKGBUILD .SRCINFO && git commit -m "wick-bin X.Y.Z-1" && git push
   ```

3. Later versions: after `bump.mjs`, copy both files again and push.

## Flathub

`io.github.abulkosim.wick` repackages the release `.deb` on the GNOME 51 runtime, which ships WebKitGTK 4.1. This is the route [Tauri's Flatpak guide](https://v2.tauri.app/distribute/flatpak/) takes, and Flathub accepts it for apps that publish their own Linux builds; building from source would need generated offline sources for every crate and npm package. The ID uses `io.github.abulkosim` because Flathub verifies `com.abulkosim` only through a website at `abulkosim.com`; the app keeps its own data folder name inside the sandbox.

`x-checker-data` lets Flathub's bot open a pull request when a release appears. The build commands were run by hand against the v0.8.0 `.deb`; `appstreamcli validate` and `desktop-file-validate` pass. `flatpak-builder` was not run here (the GNOME SDK needs several GB).

Permissions and why:

- Wayland, X11 fallback, IPC, GPU: the window.
- Network: network triggers read the host's traffic counters; Wick makes no network requests in Flatpak.
- `org.kde.StatusNotifierWatcher`, `org.kde.StatusNotifierItem-2-1`: the tray icon. The app is process 2 in the sandbox, so the name is fixed.
- `com.abulkosim.wick.SingleInstance`: one Wick at a time.
- `org.freedesktop.PowerManagement`, `ScreenSaver`, `org.gnome.SessionManager`, `org.kde.Solid.PowerManagement`, system bus `login1`, `UPower.PowerProfiles`: the inhibitors that keep the computer awake and the list of other apps holding it.

What doesn't work in the sandbox yet:

- Command triggers (`while ffmpeg runs`) can't see programs outside the sandbox.
- **Launch at login** writes its entry inside the sandbox, so it does nothing. Flatpak needs the Background portal for this.
- `wick status` and the other commands run as `flatpak run io.github.abulkosim.wick status`.

Fix Launch at login before submitting, or reviewers and users will hit it.

👤 Submit:

1. Fork [flathub/flathub](https://github.com/flathub/flathub) and branch from `new-pr`.
2. Add the three files from `packaging/flatpak/` at the top level, then build and lint locally:

   ```sh
   flatpak run --command=flathub-build org.flatpak.Builder --install io.github.abulkosim.wick.yml
   flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest io.github.abulkosim.wick.yml
   flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo
   flatpak run io.github.abulkosim.wick
   ```

3. Open a pull request against `new-pr` titled "Add io.github.abulkosim.wick". Reviewers build it with `bot, build`.
4. After merge, Flathub creates `flathub/io.github.abulkosim.wick` and invites you. Sign in at [flathub.org](https://flathub.org) with GitHub to verify the app.
5. Later versions arrive as bot pull requests there; merge them after the test build passes. Keep `packaging/flatpak/` in step with `bump.mjs`.

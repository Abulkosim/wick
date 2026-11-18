#!/usr/bin/env node
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { dmgSha256, updateCask } from "./homebrew/update-cask.mjs";

const VERSION = /^\d+\.\d+\.\d+$/;
const DATE = /^\d{4}-\d{2}-\d{2}$/;
const DOWNLOADS = "https://github.com/Abulkosim/wick/releases/download";

export const paths = {
  cask: "packaging/homebrew/Casks/wick.rb",
  scoop: "bucket/wick.json",
  winget: "packaging/winget/Abulkosim.Wick",
  pkgbuild: "packaging/aur/PKGBUILD",
  srcinfo: "packaging/aur/.SRCINFO",
  flatpak: "packaging/flatpak/io.github.abulkosim.wick.yml",
  metainfo: "packaging/flatpak/io.github.abulkosim.wick.metainfo.xml",
};

export function assets(version) {
  return {
    exe: `Wick_${version}_x64-setup.exe`,
    msi: `Wick_${version}_x64_en-US.msi`,
    debX64: `Wick_${version}_amd64.deb`,
    debArm64: `Wick_${version}_arm64.deb`,
  };
}

export function parseSums(text) {
  const sums = new Map();
  for (const line of text.split(/\r?\n/)) {
    const m = line.trim().match(/^([0-9a-f]{64})\s+\*?(.+)$/i);
    if (m) sums.set(m[2], m[1].toLowerCase());
  }
  return sums;
}

export function sumOf(sums, name) {
  const sha = sums.get(name);
  if (!sha) throw new Error(`${name} not found in SHA256SUMS.txt`);
  return sha;
}

export function releaseDate(changelog) {
  const m = changelog.match(/^(\d{4}-\d{2}-\d{2}) · /m);
  if (!m) throw new Error("Changelog has no release date line");
  return m[1];
}

function check(version, date) {
  if (!VERSION.test(version)) throw new Error(`Invalid version: ${version}`);
  if (date !== undefined && !DATE.test(date)) throw new Error(`Invalid date: ${date}`);
}

function replaceLine(source, pattern, value, what) {
  if (!pattern.test(source)) throw new Error(`No ${what} line`);
  return source.replace(pattern, (_, head) => `${head}${value}`);
}

export function updateScoop(source, version, sums) {
  check(version);
  const manifest = JSON.parse(source);
  const name = assets(version).msi;
  const arch = manifest.architecture?.["64bit"];
  if (!arch) throw new Error("Scoop manifest has no 64bit architecture");
  manifest.version = version;
  arch.url = `${DOWNLOADS}/v${version}/${name}`;
  arch.hash = sumOf(sums, name);
  return `${JSON.stringify(manifest, null, 4)}\n`;
}

export function updateWinget(files, version, sums, date) {
  check(version, date);
  const sha = sumOf(sums, assets(version).exe).toUpperCase();
  const out = {};
  for (const [name, text] of Object.entries(files)) {
    let next = replaceLine(text, /^(PackageVersion: ).*$/m, version, "PackageVersion");
    if (name.endsWith(".installer.yaml")) {
      next = replaceLine(next, /^(ReleaseDate: ).*$/m, date, "ReleaseDate");
      next = replaceLine(next, /^(\s+InstallerUrl: ).*$/m, `${DOWNLOADS}/v${version}/${assets(version).exe}`, "InstallerUrl");
      next = replaceLine(next, /^(\s+InstallerSha256: ).*$/m, sha, "InstallerSha256");
    }
    if (name.endsWith(".locale.en-US.yaml")) {
      next = replaceLine(next, /^(ReleaseNotesUrl: ).*$/m, `https://github.com/Abulkosim/wick/releases/tag/v${version}`, "ReleaseNotesUrl");
    }
    out[name] = next;
  }
  return out;
}

export function updatePkgbuild(source, version, sums, license) {
  check(version);
  const { debX64, debArm64 } = assets(version);
  let next = replaceLine(source, /^(pkgver=).*$/m, version, "pkgver");
  next = replaceLine(next, /^(pkgrel=).*$/m, "1", "pkgrel");
  next = replaceLine(next, /^(sha256sums=\(').*('\))$/m, `${license}')`, "sha256sums");
  next = replaceLine(next, /^(sha256sums_x86_64=\(').*('\))$/m, `${sumOf(sums, debX64)}')`, "sha256sums_x86_64");
  return replaceLine(next, /^(sha256sums_aarch64=\(').*('\))$/m, `${sumOf(sums, debArm64)}')`, "sha256sums_aarch64");
}

export function updateSrcinfo(source, version, sums, license) {
  check(version);
  const { debX64, debArm64 } = assets(version);
  const old = source.match(/^\tpkgver = (.+)$/m)?.[1];
  if (!old) throw new Error("No pkgver line");
  let next = source.replace(/^(\t(?:source(?:_\w+)?|noextract) = .*)$/gm, (line) => line.split(old).join(version));
  next = replaceLine(next, /^(\tpkgver = ).*$/m, version, "pkgver");
  next = replaceLine(next, /^(\tpkgrel = ).*$/m, "1", "pkgrel");
  next = replaceLine(next, /^(\tsha256sums = ).*$/m, license, "sha256sums");
  next = replaceLine(next, /^(\tsha256sums_x86_64 = ).*$/m, sumOf(sums, debX64), "sha256sums_x86_64");
  return replaceLine(next, /^(\tsha256sums_aarch64 = ).*$/m, sumOf(sums, debArm64), "sha256sums_aarch64");
}

export function updateFlatpak(source, version, sums) {
  check(version);
  let count = 0;
  const next = source.replace(
    /^(\s+url: )\S+\/Wick_[^/\s]+_(amd64|arm64)\.deb\n(\s+sha256: ).*$/gm,
    (_, urlHead, arch, shaHead) => {
      count += 1;
      const name = `Wick_${version}_${arch}.deb`;
      return `${urlHead}${DOWNLOADS}/v${version}/${name}\n${shaHead}${sumOf(sums, name)}`;
    },
  );
  if (count !== 2) throw new Error(`Expected 2 .deb sources, found ${count}`);
  return next;
}

export function updateMetainfo(source, version, date) {
  check(version, date);
  if (!/<releases>\n/.test(source)) throw new Error("Metainfo has no <releases>");
  let next = source.replace(/(raw\.githubusercontent\.com\/Abulkosim\/wick\/)v\d+\.\d+\.\d+\//g, `$1v${version}/`);
  if (next.includes(`<release version="${version}"`)) return next;
  return next.replace(
    /<releases>\n/,
    `<releases>\n    <release version="${version}" date="${date}">\n      <url type="details">https://github.com/Abulkosim/wick/releases/tag/v${version}</url>\n    </release>\n`,
  );
}

export function versions(root) {
  const read = (path) => readFileSync(join(root, path), "utf8");
  const winget = readdirSync(join(root, paths.winget));
  return {
    cask: read(paths.cask).match(/^\s*version "(.+)"$/m)?.[1],
    scoop: JSON.parse(read(paths.scoop)).version,
    winget: winget.length === 1 ? winget[0] : winget.join(","),
    pkgbuild: read(paths.pkgbuild).match(/^pkgver=(.+)$/m)?.[1],
    srcinfo: read(paths.srcinfo).match(/^\tpkgver = (.+)$/m)?.[1],
    flatpak: read(paths.flatpak).match(/\/download\/v([^/]+)\//)?.[1],
    metainfo: read(paths.metainfo).match(/<release version="([^"]+)"/)?.[1],
  };
}

export function bump(root, version, sumsText, date) {
  check(version, date);
  const sums = parseSums(sumsText);
  const read = (path) => readFileSync(join(root, path), "utf8");
  const license = createHash("sha256").update(readFileSync(join(root, "LICENSE"))).digest("hex");
  const [previous, ...extra] = readdirSync(join(root, paths.winget));
  if (!previous || extra.length) throw new Error(`Expected one version in ${paths.winget}`);
  const wingetDir = join(paths.winget, previous);
  const winget = Object.fromEntries(readdirSync(join(root, wingetDir)).map((name) => [name, read(join(wingetDir, name))]));

  const out = {
    [paths.cask]: updateCask(read(paths.cask), version, dmgSha256(sumsText, version)),
    [paths.scoop]: updateScoop(read(paths.scoop), version, sums),
    [paths.pkgbuild]: updatePkgbuild(read(paths.pkgbuild), version, sums, license),
    [paths.srcinfo]: updateSrcinfo(read(paths.srcinfo), version, sums, license),
    [paths.flatpak]: updateFlatpak(read(paths.flatpak), version, sums),
    [paths.metainfo]: updateMetainfo(read(paths.metainfo), version, date),
  };
  for (const [name, text] of Object.entries(updateWinget(winget, version, sums, date))) {
    out[join(paths.winget, version, name)] = text;
  }
  if (previous !== version) rmSync(join(root, wingetDir), { recursive: true });
  for (const [path, text] of Object.entries(out)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [rawVersion, sumsPath] = process.argv.slice(2);
  if (!rawVersion) {
    console.error("Usage: bump.mjs <version|vX.Y.Z> [SHA256SUMS.txt]");
    process.exit(2);
  }
  const version = rawVersion.replace(/^v/, "");
  const root = fileURLToPath(new URL("..", import.meta.url));
  const changelog = join(root, "changelogs", `v${version}.md`);
  if (!existsSync(changelog)) throw new Error(`Missing changelogs/v${version}.md`);
  let sums;
  if (sumsPath) sums = readFileSync(sumsPath, "utf8");
  else {
    const response = await fetch(`${DOWNLOADS}/v${version}/SHA256SUMS.txt`);
    if (!response.ok) throw new Error(`SHA256SUMS.txt for v${version}: HTTP ${response.status}. Is the release published?`);
    sums = await response.text();
  }
  bump(root, version, sums, releaseDate(readFileSync(changelog, "utf8")));
  console.log(`Homebrew, Scoop, winget, AUR and Flatpak now point at ${version}.`);
}

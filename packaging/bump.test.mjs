import assert from "node:assert/strict";
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import {
  bump,
  hasConsoleCli,
  parseSums,
  paths,
  releaseDate,
  sumOf,
  updateFlatpak,
  updateMetainfo,
  updatePkgbuild,
  updateScoop,
  updateSrcinfo,
  updateWinget,
  versions,
} from "./bump.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (path) => readFileSync(join(root, path), "utf8");
const cur = Object.values(versions(root))[0];
const next = "9.9.0";
const esc = (v) => v.replaceAll(".", "\\.");
const hex = (c) => c.repeat(64);
const names = [
  `Wick_${next}_universal.dmg`,
  `Wick_${next}_x64-setup.exe`,
  `Wick_${next}_x64_en-US.msi`,
  `Wick_${next}_amd64.deb`,
  `Wick_${next}_arm64.deb`,
];
const sumsText = names.map((name, i) => `${hex("abcde"[i])}  ${name}`).join("\n");
const sums = parseSums(sumsText);
const winget = () => {
  const dir = join(paths.winget, cur);
  return Object.fromEntries(readdirSync(join(root, dir)).map((name) => [name, read(join(dir, name))]));
};

function pkgbuildFields(source, version) {
  const fields = {};
  for (const m of source.matchAll(/^(\w+)=(\((?:[^)]*)\)|.*)$/gm)) {
    const raw = m[2].startsWith("(") ? [...m[2].matchAll(/'([^']*)'|"([^"]*)"/g)].map((x) => x[1] ?? x[2]) : [m[2].replace(/^'|'$/g, "")];
    fields[m[1]] = raw.map((v) =>
      v.replaceAll("${pkgver}", version).replaceAll("$pkgver", version).replaceAll("$url", fields.url?.[0] ?? ""),
    );
  }
  return fields;
}

function srcinfoFields(source) {
  const fields = {};
  for (const m of source.matchAll(/^\t(\w+) = (.*)$/gm)) (fields[m[1]] ??= []).push(m[2]);
  return fields;
}

test("reads checksums and the release date", () => {
  assert.equal(sumOf(sums, `Wick_${next}_arm64.deb`), hex("e"));
  assert.throws(() => sumOf(sums, `Wick_${next}_x64.zip`), /not found/);
  assert.equal(releaseDate("# v0.9.0\n\n2026-12-01 · [Download](https://example.com)\n"), "2026-12-01");
  assert.throws(() => releaseDate("# v0.9.0\n"), /no release date/);
});

test("every channel points at the same published version", () => {
  const found = versions(root);
  assert.equal(new Set(Object.values(found)).size, 1, JSON.stringify(found));
});

test("Scoop gets the MSI and keeps checkver and autoupdate", () => {
  const before = JSON.parse(read(paths.scoop));
  const after = JSON.parse(updateScoop(read(paths.scoop), next, sums));
  assert.equal(after.version, next);
  assert.equal(after.architecture["64bit"].url, `https://github.com/Abulkosim/wick/releases/download/v${next}/Wick_${next}_x64_en-US.msi`);
  assert.equal(after.architecture["64bit"].hash, hex("c"));
  assert.deepEqual(after.autoupdate, before.autoupdate);
  assert.deepEqual(after.checkver, before.checkver);
  assert.match(before.post_install, /package-manager.*'Scoop'/);
});

test("Scoop shims wick to the console wick-cli.exe from the release that ships it", () => {
  const sumsNext = new Map(["0.9.0", "0.9.1", "0.10.0"].map((version) => [`Wick_${version}_x64_en-US.msi`, hex("c")]));
  const old = { ...JSON.parse(read(paths.scoop)), bin: undefined };
  assert.equal(JSON.parse(updateScoop(JSON.stringify(old), "0.9.0", sumsNext)).bin, undefined);
  for (const version of ["0.9.1", "0.10.0"]) {
    const after = JSON.parse(updateScoop(read(paths.scoop), version, sumsNext));
    assert.deepEqual(after.bin, [["wick-cli.exe", "wick"]]);
    assert.equal(Object.keys(after).indexOf("bin") + 1, Object.keys(after).indexOf("shortcuts"));
    assert.deepEqual(JSON.parse(updateScoop(JSON.stringify(after), version, sumsNext)), after);
  }
  assert.equal(hasConsoleCli("0.9.0"), false);
  assert.equal(hasConsoleCli("0.8.3"), false);
  assert.equal(hasConsoleCli("1.0.0"), true);
});

test("winget gets the setup EXE, release date and notes link", () => {
  const before = winget();
  const after = updateWinget(before, next, sums, "2026-12-01");
  assert.deepEqual(Object.keys(after), Object.keys(before));
  for (const [name, text] of Object.entries(after)) {
    assert.match(text, new RegExp(`^PackageVersion: ${esc(next)}$`, "m"));
    assert.doesNotMatch(text, new RegExp(esc(cur)), name);
    assert.equal(text.split("\n").length, before[name].split("\n").length);
  }
  const installer = after["Abulkosim.Wick.installer.yaml"];
  assert.match(installer, /^ReleaseDate: 2026-12-01$/m);
  assert.match(installer, new RegExp(`InstallerUrl: .*/v${esc(next)}/Wick_${esc(next)}_x64-setup\\.exe$`, "m"));
  assert.match(installer, new RegExp(`InstallerSha256: ${hex("B")}$`, "m"));
  assert.throws(() => updateWinget(before, next, sums, "1 Dec"), /Invalid date/);
});

test(".SRCINFO matches the PKGBUILD, before and after a bump", () => {
  const license = hex("f");
  for (const [pkgbuild, srcinfo, version] of [
    [read(paths.pkgbuild), read(paths.srcinfo), cur],
    [updatePkgbuild(read(paths.pkgbuild), next, sums, license), updateSrcinfo(read(paths.srcinfo), next, sums, license), next],
  ]) {
    const p = pkgbuildFields(pkgbuild, version);
    const s = srcinfoFields(srcinfo);
    for (const key of ["pkgdesc", "pkgver", "pkgrel", "url", "arch", "license", "depends", "provides", "conflicts", "options", "source", "source_x86_64", "source_aarch64", "sha256sums", "sha256sums_x86_64", "sha256sums_aarch64"]) {
      assert.deepEqual(s[key], p[key], key);
    }
    assert.match(srcinfo, /^pkgbase = wick-bin$/m);
    assert.match(srcinfo, /^pkgname = wick-bin$/m);
  }
  const bumped = srcinfoFields(updateSrcinfo(read(paths.srcinfo), next, sums, license));
  assert.deepEqual(bumped.sha256sums_x86_64, [hex("d")]);
  assert.deepEqual(bumped.sha256sums_aarch64, [hex("e")]);
  assert.match(read(paths.pkgbuild), /usr\/lib\/wick\/package-manager/);
});

test("Flatpak gets both .deb files", () => {
  const out = updateFlatpak(read(paths.flatpak), next, sums);
  assert.match(out, new RegExp(`/v${esc(next)}/Wick_${esc(next)}_amd64\\.deb\\n\\s+sha256: ${hex("d")}`));
  assert.match(out, new RegExp(`/v${esc(next)}/Wick_${esc(next)}_arm64\\.deb\\n\\s+sha256: ${hex("e")}`));
  assert.doesNotMatch(out, new RegExp(esc(cur)));
  assert.throws(() => updateFlatpak("modules: []\n", next, sums), /Expected 2/);
});

test("metainfo gets the new release on top, once, and screenshots from its tag", () => {
  const list = (xml) => [...xml.matchAll(/<release version="([^"]+)" date="([^"]+)"/g)].map((m) => `${m[1]} ${m[2]}`);
  const before = list(read(paths.metainfo));
  assert.equal(before[0].split(" ")[0], cur);
  const once = updateMetainfo(read(paths.metainfo), next, "2026-12-01");
  assert.equal(updateMetainfo(once, next, "2026-12-01"), once);
  assert.deepEqual(list(once), [`${next} 2026-12-01`, ...before]);
  assert.doesNotMatch(once, new RegExp(`wick/v${esc(cur)}/docs`));
  assert.match(once, new RegExp(`wick/v${esc(next)}/docs/images/`));
});

test("one bump rewrites every channel", () => {
  const dir = mkdtempSync(join(tmpdir(), "wick-bump-"));
  try {
    for (const path of ["LICENSE", "packaging", "bucket"]) cpSync(join(root, path), join(dir, path), { recursive: true });
    bump(dir, next, sumsText, "2026-12-01");
    const found = versions(dir);
    assert.deepEqual(new Set(Object.values(found)), new Set([next]), JSON.stringify(found));
    assert.equal(existsSync(join(dir, paths.winget, cur)), false);
    assert.match(readFileSync(join(dir, paths.cask), "utf8"), new RegExp(`sha256 "${hex("a")}"`));
    assert.throws(() => bump(dir, "9.9.1", sumsText, "2026-12-02"), /not found/);
    assert.deepEqual(versions(dir), found);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

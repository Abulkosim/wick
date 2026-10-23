#![cfg(target_os = "macos")]
//! Installs the live release over an existing Wick.app through the updater plugin, exactly as
//! Settings → Updates would, using the public endpoint and key from tauri.conf.json.
//!
//! WICK_UPDATE_TARGET=/Applications/Wick.app cargo test --test public_update -- --ignored --nocapture

use std::{path::PathBuf, process::Command, time::Duration};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::UpdaterExt;

fn bundle_version(app: &std::path::Path) -> String {
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
        .arg(app.join("Contents/Info.plist"))
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "installs the published release over WICK_UPDATE_TARGET"]
async fn install_the_published_release() {
    let app_path = PathBuf::from(
        std::env::var_os("WICK_UPDATE_TARGET").expect("Set WICK_UPDATE_TARGET to a Wick.app"),
    );
    let before = bundle_version(&app_path);
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let mut context = mock_context(noop_assets());
    context.package_info_mut().version = before.parse().unwrap();
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), config["plugins"]["updater"].clone());
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let update = app
        .updater_builder()
        .executable_path(app_path.join("Contents/MacOS/wick"))
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
        .check()
        .await
        .unwrap()
        .expect("The public manifest should offer a newer version");
    let version = update.version.clone();
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    tokio::task::spawn_blocking(move || update.install(bytes))
        .await
        .unwrap()
        .unwrap();
    let after = bundle_version(&app_path);
    assert_eq!(after, version);
    let verified = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&app_path)
        .status()
        .unwrap();
    assert!(verified.success());
    println!("Updated {} from {before} to {after}.", app_path.display());
}

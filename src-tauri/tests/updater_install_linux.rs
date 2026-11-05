#![cfg(target_os = "linux")]

mod support;

use serde_json::json;
use std::{fs, path::PathBuf, time::Duration};
use support::ReleaseServer;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::{Error, UpdaterExt};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the exact signed AppImage in WICK_UPDATE_ARTIFACT_DIR"]
async fn signed_appimage_replaces_a_disposable_one() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let version = config["version"].as_str().unwrap();
    let public_key = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
    assert_eq!(config["plugins"]["updater"]["requireSignedVersion"], true);
    let file = format!("Wick_{version}_amd64.AppImage");
    let artifact_directory = PathBuf::from(
        std::env::var_os("WICK_UPDATE_ARTIFACT_DIR")
            .expect("Set WICK_UPDATE_ARTIFACT_DIR to the release assets"),
    );
    let artifact = fs::read(artifact_directory.join(&file)).unwrap();
    let signature = fs::read_to_string(artifact_directory.join(format!("{file}.sig"))).unwrap();
    assert!(artifact.starts_with(b"\x7fELF"), "not an AppImage");
    let server = ReleaseServer::start(
        version,
        signature.trim(),
        &artifact,
        &file,
        &["linux-x86_64", "linux-x86_64-appimage"],
    );

    let disposable = tempfile::tempdir().unwrap();
    let appimage = disposable.path().join("Wick.AppImage");
    let old = b"\x7fELF disposable old AppImage".to_vec();
    fs::write(&appimage, &old).unwrap();
    // A real AppImage is executable, and the updater keeps the old file's permissions.
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&appimage, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut context = mock_context(noop_assets());
    context.package_info_mut().version = "0.0.0".parse().unwrap();
    context.config_mut().plugins.0.insert(
        "updater".into(),
        json!({
            "pubkey": public_key,
            "requireSignedVersion": true,
            "dangerousInsecureTransportProtocol": true,
            "endpoints": [server.url("/latest.json")]
        }),
    );
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    let updater = |path: &str| {
        app.updater_builder()
            .executable_path(appimage.clone())
            .endpoints(vec![server.url(path)])
            .unwrap()
            .no_proxy()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap()
    };

    let update = updater("/latest.json").check().await.unwrap().unwrap();
    assert_eq!(update.version, version);
    let downloaded = update.download(|_, _| {}, || {}).await.unwrap();
    assert_eq!(downloaded, artifact);

    let tampered = updater("/tampered.json").check().await.unwrap().unwrap();
    assert!(matches!(
        tampered.download(|_, _| {}, || {}).await,
        Err(Error::Minisign(_))
    ));
    let mut altered = artifact.clone();
    *altered.last_mut().unwrap() ^= 1;
    assert!(matches!(update.install(altered), Err(Error::Minisign(_))));
    assert_eq!(fs::read(&appimage).unwrap(), old);

    let mismatch = updater("/mismatch.json").check().await.unwrap().unwrap();
    assert!(matches!(
        mismatch.download(|_, _| {}, || {}).await,
        Err(Error::SignedVersionMismatch { .. })
    ));
    assert_eq!(fs::read(&appimage).unwrap(), old);

    tokio::task::spawn_blocking(move || update.install(downloaded))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fs::read(&appimage).unwrap(), artifact);
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&appimage).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "the new AppImage isn't executable");
    }
    println!(
        "Verified signature failures, signed-version failures, and the {version} AppImage install."
    );
}

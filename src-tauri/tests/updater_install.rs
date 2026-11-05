#![cfg(target_os = "macos")]

mod support;

use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use support::ReleaseServer;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::{Error, UpdaterExt};

fn command_output(program: &str, arguments: &[&str], path: &Path) -> String {
    let output = Command::new(program)
        .args(arguments)
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the exact signed macOS release archive in WICK_UPDATE_ARTIFACT_DIR"]
async fn signed_release_installs_into_a_disposable_app() {
    let artifact_directory = PathBuf::from(
        std::env::var_os("WICK_UPDATE_ARTIFACT_DIR")
            .expect("Set WICK_UPDATE_ARTIFACT_DIR to downloaded release assets"),
    );
    let artifact = fs::read(artifact_directory.join("Wick.app.tar.gz")).unwrap();
    let signature = fs::read_to_string(artifact_directory.join("Wick.app.tar.gz.sig")).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let version = config["version"].as_str().unwrap();
    let public_key = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
    assert_eq!(config["plugins"]["updater"]["requireSignedVersion"], true);
    let server = ReleaseServer::start(
        version,
        signature.trim(),
        &artifact,
        "Wick.app.tar.gz",
        &["darwin-aarch64", "darwin-x86_64"],
    );

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".cache/upgrade-smoke");
    fs::create_dir_all(&root).unwrap();
    let disposable = tempfile::Builder::new()
        .prefix("signed-install-")
        .tempdir_in(root)
        .unwrap();
    let app_path = disposable.path().join("Wick.app");
    let executable = app_path.join("Contents/MacOS/wick");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, "disposable old executable").unwrap();
    let sentinel = app_path.join("old-version-sentinel.txt");
    fs::write(&sentinel, "old app must survive rejected updates").unwrap();

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
            .executable_path(executable.clone())
            .endpoints(vec![server.url(path)])
            .unwrap()
            .no_proxy()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap()
    };

    let update = updater("/latest.json").check().await.unwrap().unwrap();
    assert_eq!(update.version, version);
    let downloaded = update.download(|_, _| {}, || {}).await.unwrap();
    assert_eq!(downloaded, artifact);

    let tampered_update = updater("/tampered.json").check().await.unwrap().unwrap();
    assert!(matches!(
        tampered_update.download(|_, _| {}, || {}).await,
        Err(Error::Minisign(_))
    ));
    let mut tampered = artifact.clone();
    *tampered.last_mut().unwrap() ^= 1;
    assert!(matches!(update.install(tampered), Err(Error::Minisign(_))));
    assert_eq!(
        fs::read_to_string(&sentinel).unwrap(),
        "old app must survive rejected updates"
    );

    let mismatch = updater("/mismatch.json").check().await.unwrap().unwrap();
    assert!(matches!(
        mismatch.download(|_, _| {}, || {}).await,
        Err(Error::SignedVersionMismatch { .. })
    ));
    assert!(matches!(
        mismatch.install(&artifact),
        Err(Error::SignedVersionMismatch { .. })
    ));
    assert!(sentinel.exists());

    tokio::task::spawn_blocking(move || update.install(downloaded))
        .await
        .unwrap()
        .unwrap();
    assert!(!sentinel.exists());
    assert!(executable.is_file());
    let installed_version = command_output(
        "/usr/bin/plutil",
        &["-extract", "CFBundleShortVersionString", "raw", "-o", "-"],
        &app_path.join("Contents/Info.plist"),
    );
    assert_eq!(installed_version, version);
    command_output(
        "/usr/bin/codesign",
        &["--verify", "--deep", "--strict"],
        &app_path,
    );
    let architectures = command_output("/usr/bin/lipo", &["-archs"], &executable);
    let current_architecture = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };
    assert!(architectures
        .split_whitespace()
        .any(|arch| arch == current_architecture));
    if std::env::var("WICK_REQUIRE_UNIVERSAL").as_deref() == Ok("1") {
        assert!(architectures.split_whitespace().any(|arch| arch == "arm64"));
        assert!(architectures
            .split_whitespace()
            .any(|arch| arch == "x86_64"));
    }
    let retained = disposable.keep();
    println!(
        "Verified signature failures, signed-version failures, and {version} install ({architectures}). Disposable app retained: {}",
        retained.join("Wick.app").display()
    );
}

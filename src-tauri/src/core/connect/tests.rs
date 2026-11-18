use super::*;
use serde_json::json;

struct Home {
    _dir: tempfile::TempDir,
    root: PathBuf,
    places: Places,
    exe: String,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let exe = root.join("My Apps/Wick.app/Contents/MacOS/wick");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, "").unwrap();
        let exe = exe.display().to_string();
        let places = Places {
            home: Some(root.clone()),
            claude: Some(root.join(".claude")),
            codex: Some(root.join(".codex")),
            exe: Ok(exe.clone()),
            windows: false,
        };
        Self {
            _dir: dir,
            root,
            places,
            exe,
        }
    }

    fn claude(&self, text: Option<&str>) -> PathBuf {
        let directory = self.root.join(".claude");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        if let Some(text) = text {
            fs::write(&path, text).unwrap();
        }
        path
    }

    fn codex(&self, config: Option<&str>, hooks: Option<&str>) -> (PathBuf, PathBuf) {
        let directory = self.root.join(".codex");
        fs::create_dir_all(&directory).unwrap();
        let paths = (directory.join("config.toml"), directory.join("hooks.json"));
        if let Some(config) = config {
            fs::write(&paths.0, config).unwrap();
        }
        if let Some(hooks) = hooks {
            fs::write(&paths.1, hooks).unwrap();
        }
        paths
    }

    fn status(&self, agent: Agent) -> Connection {
        connection(&self.places, agent)
    }

    fn command(&self, agent: Agent) -> String {
        hook_command(&self.exe, agent, false)
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn backup(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(".wick-backup");
    path.with_file_name(name)
}

fn parsed(path: &Path) -> serde_json::Value {
    serde_json::from_str(&read(path)).unwrap()
}

fn pretty_value(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}

/// A settings file as Claude Code writes it, keys out of alphabetical order.
const CLAUDE_SETTINGS: &str = r#"{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "model": "opus",
  "permissions": {
    "allow": [
      "Bash(git status:*)"
    ],
    "deny": []
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/guard.sh"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "say done",
            "timeout": 10
          }
        ]
      }
    ]
  },
  "statusLine": {
    "type": "command",
    "command": "~/.claude/status.sh"
  },
  "env": {
    "ZED": "1",
    "ALPHA": "2.5e-3"
  }
}
"#;

#[test]
fn hook_commands_quote_paths_for_each_shell() {
    let mac = "/Applications/Wick.app/Contents/MacOS/wick";
    assert_eq!(
        hook_command(mac, Agent::Claude, false),
        r#""/Applications/Wick.app/Contents/MacOS/wick" hook claude"#
    );
    let odd = r#"/home/me/My "Apps"/$HOME/`x`\wick"#;
    let command = hook_command(odd, Agent::Codex, false);
    assert_eq!(
        command,
        r#""/home/me/My \"Apps\"/\$HOME/\`x\`\\wick" hook codex"#
    );
    assert_eq!(
        wick_binary(&command, Agent::Codex, false).as_deref(),
        Some(odd)
    );
    let windows = r"C:\Program Files\Wick\wick.exe";
    let command = hook_command(windows, Agent::Claude, true);
    assert_eq!(command, r#""C:/Program Files/Wick/wick.exe" hook claude"#);
    assert_eq!(
        wick_binary(&command, Agent::Claude, true).as_deref(),
        Some("C:/Program Files/Wick/wick.exe")
    );
    assert_eq!(
        wick_binary(
            r#""C:\Users\Me\AppData\Local\Wick\wick.exe" hook claude"#,
            Agent::Claude,
            true
        )
        .as_deref(),
        Some(r"C:\Users\Me\AppData\Local\Wick\wick.exe")
    );
    assert_eq!(
        wick_binary("/usr/bin/wick hook claude", Agent::Claude, false).as_deref(),
        Some("/usr/bin/wick")
    );
    assert_eq!(
        wick_binary(
            r#""/home/me/Wick_0.9.0_amd64.AppImage" hook codex"#,
            Agent::Codex,
            false
        )
        .as_deref(),
        Some("/home/me/Wick_0.9.0_amd64.AppImage")
    );
    for other in [
        r#""/usr/bin/wick" hook codex"#,
        r#""/usr/bin/wicked" hook claude"#,
        r#""/usr/bin/notwick" hook claude"#,
        r#""/usr/bin/wick" hook claude --verbose"#,
        "~/.claude/guard.sh",
    ] {
        assert_eq!(wick_binary(other, Agent::Claude, false), None, "{other}");
    }
}

#[test]
fn trust_hashes_match_what_codex_wrote() {
    // Written by Codex 0.166's "Trust all and continue" for hooks without a timeout.
    let command = "/tmp/claude-1000/-home-dev-work-wick/08d9db3b-d4aa-4f99-8211-a1bee137b1ec/scratchpad/bin/log-hook.sh";
    assert_eq!(
        trust_hash(
            "session_start",
            &format!("{command} SessionStart"),
            None,
            None
        ),
        "sha256:a4dc3c83d0ec4b5afacc80d4d74499fa167f296cbc0d7d57d9d01df537dfc29a"
    );
    assert_eq!(
        trust_hash("session_end", &format!("{command} SessionEnd"), None, None),
        "sha256:3b0b220da6fa8aa56dc6cfc1db65c078fb3311132caae42a8c51941653017477"
    );
    assert_eq!(snake_case("UserPromptSubmit"), "user_prompt_submit");
    assert_eq!(snake_case("Stop"), "stop");
}

#[test]
fn claude_connect_keeps_everything_else_and_backs_up_once() {
    let home = Home::new();
    let path = home.claude(Some(CLAUDE_SETTINGS));
    assert_eq!(home.status(Agent::Claude).status, Status::Disconnected);
    assert_eq!(home.status(Agent::Claude).file, "~/.claude/settings.json");

    connect(&home.places, Agent::Claude).unwrap();
    let text = read(&path);
    assert!(text.ends_with("}\n") && text.starts_with("{\n  \"$schema\""));
    let keys: Vec<String> = serde_json::from_str::<Json>(&text)
        .unwrap()
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, _)| key.to_string())
        .collect();
    assert_eq!(
        keys,
        [
            "$schema",
            "model",
            "permissions",
            "hooks",
            "statusLine",
            "env"
        ]
    );
    assert!(text.find("\"ZED\"").unwrap() < text.find("\"ALPHA\"").unwrap());
    let value = parsed(&path);
    let ours = json!({ "hooks": [{ "type": "command", "command": home.command(Agent::Claude) }] });
    assert_eq!(value["hooks"]["PreToolUse"][0]["matcher"], "Bash");
    assert_eq!(value["hooks"]["PreToolUse"][1], ours);
    assert_eq!(value["hooks"]["Stop"][0]["hooks"][0]["command"], "say done");
    assert_eq!(value["hooks"]["Stop"][1], ours);
    for (event, _) in CLAUDE_EVENTS {
        let groups = value["hooks"][event].as_array().unwrap();
        assert_eq!(groups.last().unwrap(), &ours, "{event}");
    }
    assert_eq!(
        value["hooks"].as_object().unwrap().len(),
        CLAUDE_EVENTS.len()
    );
    assert_eq!(read(&backup(&path)), CLAUDE_SETTINGS);
    assert_eq!(home.status(Agent::Claude).status, Status::Connected);

    connect(&home.places, Agent::Claude).unwrap();
    assert_eq!(read(&path), text, "connecting twice changes nothing");

    disconnect(&home.places, Agent::Claude).unwrap();
    assert_eq!(read(&path), CLAUDE_SETTINGS, "disconnect undoes connect");
    assert_eq!(home.status(Agent::Claude).status, Status::Disconnected);
    disconnect(&home.places, Agent::Claude).unwrap();
    assert_eq!(read(&path), CLAUDE_SETTINGS);

    fs::write(&path, CLAUDE_SETTINGS.replace("opus", "sonnet")).unwrap();
    connect(&home.places, Agent::Claude).unwrap();
    assert_eq!(
        read(&backup(&path)),
        CLAUDE_SETTINGS,
        "the first backup stays"
    );
}

#[test]
fn claude_connect_adds_and_removes_the_hooks_key_it_created() {
    let home = Home::new();
    let original = "{\n  \"model\": \"opus\"\n}\n";
    let path = home.claude(Some(original));
    connect(&home.places, Agent::Claude).unwrap();
    assert_eq!(
        parsed(&path)["hooks"].as_object().unwrap().len(),
        CLAUDE_EVENTS.len()
    );
    disconnect(&home.places, Agent::Claude).unwrap();
    assert_eq!(read(&path), original);
}

#[test]
fn claude_connect_creates_a_missing_settings_file() {
    let home = Home::new();
    let path = home.claude(None);
    assert_eq!(home.status(Agent::Claude).status, Status::Disconnected);
    connect(&home.places, Agent::Claude).unwrap();
    assert_eq!(home.status(Agent::Claude).status, Status::Connected);
    assert!(!backup(&path).exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    disconnect(&home.places, Agent::Claude).unwrap();
    assert!(!path.exists(), "a file Wick created goes with its hooks");
}

#[test]
fn invalid_json_is_left_alone_with_an_error_naming_the_file() {
    let home = Home::new();
    let broken = "{ \"model\": \"opus\", }\n";
    let path = home.claude(Some(broken));
    let message = "~/.claude/settings.json isn’t valid JSON. Wick left it unchanged.";
    let status = home.status(Agent::Claude);
    assert_eq!(status.status, Status::Error);
    assert_eq!(status.error.as_deref(), Some(message));
    assert_eq!(connect(&home.places, Agent::Claude).unwrap_err(), message);
    assert!(disconnect(&home.places, Agent::Claude).is_err());
    assert_eq!(read(&path), broken);
    assert!(!backup(&path).exists());

    fs::write(&path, "{\"hooks\": []}").unwrap();
    let error = connect(&home.places, Agent::Claude).unwrap_err();
    assert!(error.contains("hooks Wick can’t read"), "{error}");
    assert_eq!(read(&path), "{\"hooks\": []}");
    fs::write(&path, "[1]").unwrap();
    assert!(connect(&home.places, Agent::Claude).is_err());
    assert_eq!(read(&path), "[1]");
}

#[test]
fn missing_agents_are_not_installed() {
    let home = Home::new();
    let found: Vec<_> = connections(&home.places)
        .into_iter()
        .map(|connection| (connection.agent, connection.status, connection.file))
        .collect();
    assert_eq!(
        found,
        [
            (
                Agent::Claude,
                Status::Missing,
                "~/.claude/settings.json".to_string()
            ),
            (
                Agent::Codex,
                Status::Missing,
                "~/.codex/hooks.json".to_string()
            ),
        ]
    );
    assert_eq!(
        connect(&home.places, Agent::Claude).unwrap_err(),
        "Claude Code isn’t installed."
    );
    assert_eq!(
        connect(&home.places, Agent::Codex).unwrap_err(),
        "Codex isn’t installed."
    );
    disconnect(&home.places, Agent::Claude).unwrap();
    assert!(!home.root.join(".claude").exists());
}

#[test]
fn a_moved_app_is_stale_and_reconnect_rewrites_in_place() {
    let home = Home::new();
    let gone = hook_command(
        "/Volumes/Wick/Wick.app/Contents/MacOS/wick",
        Agent::Claude,
        false,
    );
    let original = pretty_value(&json!({
        "hooks": {
            "Stop": [
                { "hooks": [{ "type": "command", "command": gone }] },
                { "hooks": [{ "type": "command", "command": "say done" }] }
            ]
        }
    }));
    let path = home.claude(Some(&original));
    assert_eq!(home.status(Agent::Claude).status, Status::Stale);
    connect(&home.places, Agent::Claude).unwrap();
    let value = parsed(&path);
    let stop = value["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 2);
    assert_eq!(stop[0]["hooks"][0]["command"], home.command(Agent::Claude));
    assert_eq!(stop[1]["hooks"][0]["command"], "say done");
    assert!(!read(&path).contains("/Volumes/Wick"));
    assert_eq!(home.status(Agent::Claude).status, Status::Connected);

    let mut partial = parsed(&path);
    partial["hooks"]
        .as_object_mut()
        .unwrap()
        .remove("SessionEnd");
    fs::write(&path, partial.to_string()).unwrap();
    let status = home.status(Agent::Claude);
    assert_eq!(status.status, Status::Error);
    assert_eq!(
        status.error.as_deref(),
        Some("Some of Wick’s hooks are missing. Connect again.")
    );
    connect(&home.places, Agent::Claude).unwrap();
    assert_eq!(home.status(Agent::Claude).status, Status::Connected);
}

#[test]
fn other_hooks_that_look_like_wick_are_kept() {
    let home = Home::new();
    let mixed = json!({
        "hooks": {
            "Stop": [
                { "hooks": [{ "type": "command", "command": "\"/usr/bin/notwick\" hook claude" }] },
                { "hooks": [{ "type": "command", "command": "\"/usr/bin/wick\" hook codex" }] },
                { "hooks": [
                    { "type": "command", "command": "\"/usr/bin/wick\" hook claude" },
                    { "type": "command", "command": "say done" }
                ] }
            ]
        }
    });
    let path = home.claude(Some(&mixed.to_string()));
    assert_eq!(home.status(Agent::Claude).status, Status::Disconnected);
    connect(&home.places, Agent::Claude).unwrap();
    assert_eq!(parsed(&path)["hooks"]["Stop"].as_array().unwrap().len(), 4);
    disconnect(&home.places, Agent::Claude).unwrap();
    assert_eq!(parsed(&path), mixed);
}

#[cfg(unix)]
#[test]
fn a_symlinked_settings_file_stays_a_symlink() {
    let home = Home::new();
    let dotfiles = home.root.join("dotfiles");
    fs::create_dir_all(&dotfiles).unwrap();
    let real = dotfiles.join("claude.json");
    fs::write(&real, "{}\n").unwrap();
    let path = home.claude(None);
    std::os::unix::fs::symlink(&real, &path).unwrap();
    connect(&home.places, Agent::Claude).unwrap();
    assert!(fs::symlink_metadata(&path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(read(&real).contains(" hook claude"));
    assert_eq!(read(&backup(&path)), "{}\n");
}

const CODEX_CONFIG: &str = r#"# My Codex settings
model = "gpt-6.1-sol"   # pinned

[projects."/home/me/work"]
trust_level = "trusted"

[hooks.state.'HOOKS:stop:0:0']
trusted_hash = "sha256:user-stop"

[hooks.state.'HOOKS:pre_tool_use:0:0']
trusted_hash = "sha256:user-pre"
enabled = true
"#;

const CODEX_HOOKS: &str = r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "notify-send done"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "~/.codex/guard.sh"
          }
        ]
      }
    ]
  }
}
"#;

fn codex_trust(config: &Path) -> Vec<(String, String)> {
    let document: DocumentMut = read(config).parse().unwrap();
    let Some(state) = document
        .get("hooks")
        .and_then(Item::as_table_like)
        .and_then(|hooks| hooks.get("state"))
        .and_then(Item::as_table_like)
    else {
        return Vec::new();
    };
    state
        .iter()
        .map(|(key, item)| {
            let hash = item.as_table_like().unwrap().get("trusted_hash").unwrap();
            (key.to_string(), hash.as_str().unwrap().to_string())
        })
        .collect()
}

#[test]
fn codex_connect_appends_hooks_and_trusts_only_them() {
    let home = Home::new();
    let (config, hooks) = home.codex(None, Some(CODEX_HOOKS));
    let key = hooks.display().to_string();
    let original_config = CODEX_CONFIG.replace("HOOKS", &key);
    fs::write(&config, &original_config).unwrap();
    assert_eq!(home.status(Agent::Codex).status, Status::Disconnected);
    assert_eq!(home.status(Agent::Codex).file, "~/.codex/hooks.json");

    connect(&home.places, Agent::Codex).unwrap();
    let value = parsed(&hooks);
    let command = home.command(Agent::Codex);
    assert_eq!(
        value["hooks"]["Stop"][0]["hooks"][0]["command"],
        "notify-send done"
    );
    assert_eq!(
        value["hooks"]["Stop"][1],
        json!({ "hooks": [{ "type": "command", "command": command, "timeout": 5 }] })
    );
    assert_eq!(value["hooks"]["SessionEnd"][0]["hooks"][0]["timeout"], 1);
    assert_eq!(value["hooks"]["Interrupt"][0]["hooks"][0]["timeout"], 1);
    assert_eq!(
        value["hooks"].as_object().unwrap().len(),
        CODEX_EVENTS.len()
    );
    let text = read(&config);
    assert!(text.starts_with(&original_config), "{text}");
    let trust = codex_trust(&config);
    assert_eq!(
        trust[0],
        (format!("{key}:stop:0:0"), "sha256:user-stop".into())
    );
    assert_eq!(
        trust[1],
        (format!("{key}:pre_tool_use:0:0"), "sha256:user-pre".into())
    );
    assert!(trust.contains(&(
        format!("{key}:stop:1:0"),
        trust_hash("stop", &command, Some(5), None)
    )));
    assert!(trust.contains(&(
        format!("{key}:session_end:0:0"),
        trust_hash("session_end", &command, Some(1), None)
    )));
    assert_eq!(trust.len(), 2 + CODEX_EVENTS.len());
    assert_eq!(read(&backup(&config)), original_config);
    assert_eq!(read(&backup(&hooks)), CODEX_HOOKS);
    assert_eq!(home.status(Agent::Codex).status, Status::Connected);

    let (hooks_text, config_text) = (read(&hooks), read(&config));
    connect(&home.places, Agent::Codex).unwrap();
    assert_eq!((read(&hooks), read(&config)), (hooks_text, config_text));

    disconnect(&home.places, Agent::Codex).unwrap();
    assert_eq!(read(&hooks), CODEX_HOOKS);
    assert_eq!(read(&config), original_config);
    assert_eq!(home.status(Agent::Codex).status, Status::Disconnected);
}

#[test]
fn codex_disconnect_moves_the_trust_of_later_hooks() {
    let home = Home::new();
    let (config, hooks) = home.codex(Some(""), Some(CODEX_HOOKS));
    let key = hooks.display().to_string();
    connect(&home.places, Agent::Codex).unwrap();
    let mut value = parsed(&hooks);
    value["hooks"]["Stop"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "hooks": [{ "type": "command", "command": "afterwards" }] }));
    fs::write(&hooks, pretty_value(&value)).unwrap();
    let mut text = read(&config);
    text.push_str(&format!(
        "\n[hooks.state.'{key}:stop:2:0']\ntrusted_hash = \"sha256:later\"\n"
    ));
    fs::write(&config, text).unwrap();
    assert_eq!(home.status(Agent::Codex).status, Status::Connected);

    disconnect(&home.places, Agent::Codex).unwrap();
    let value = parsed(&hooks);
    let stop = value["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 2);
    assert_eq!(stop[1]["hooks"][0]["command"], "afterwards");
    assert_eq!(
        codex_trust(&config),
        [(format!("{key}:stop:1:0"), "sha256:later".to_string())]
    );
}

#[test]
fn codex_without_trust_or_with_hooks_off_asks_to_connect_again() {
    let home = Home::new();
    let (config, _) = home.codex(None, None);
    connect(&home.places, Agent::Codex).unwrap();
    let mut document: DocumentMut = read(&config).parse().unwrap();
    let state = document["hooks"]["state"].as_table_like_mut().unwrap();
    let key = state
        .iter()
        .map(|(key, _)| key.to_string())
        .find(|key| key.contains(":stop:"))
        .unwrap();
    state.remove(&key);
    fs::write(&config, document.to_string()).unwrap();
    let status = home.status(Agent::Codex);
    assert_eq!(status.status, Status::Error);
    assert_eq!(
        status.error.as_deref(),
        Some("Codex doesn’t trust Wick’s hooks yet. Connect again.")
    );
    connect(&home.places, Agent::Codex).unwrap();
    assert_eq!(home.status(Agent::Codex).status, Status::Connected);

    let mut text = read(&config);
    text.push_str("\n[features]\nhooks = false\n");
    fs::write(&config, text).unwrap();
    let status = home.status(Agent::Codex);
    assert_eq!(status.status, Status::Connected);
    assert_eq!(
        status.error.as_deref(),
        Some("Hooks are turned off in ~/.codex/config.toml.")
    );
    let before = read(&config);
    assert_eq!(
        connect(&home.places, Agent::Codex).unwrap_err(),
        "Hooks are turned off in ~/.codex/config.toml. Turn them on, then connect."
    );
    assert_eq!(read(&config), before);
    disconnect(&home.places, Agent::Codex).unwrap();
    let status = home.status(Agent::Codex);
    assert_eq!(status.status, Status::Error);
    assert_eq!(
        status.error.as_deref(),
        Some("Hooks are turned off in ~/.codex/config.toml.")
    );
    assert!(!read(&config).contains("trusted_hash"));
}

#[test]
fn windows_paths_that_shells_expand_are_refused() {
    let plain = r"C:\Users\Me\AppData\Local\Wick\wick.exe".to_string();
    assert_eq!(runnable(plain.clone(), true), Ok(plain));
    for odd in [
        r"C:\Users\svc$admin\Wick\wick.exe",
        r"D:\`x`\wick.exe",
        r"D:\100%\wick.exe",
    ] {
        assert_eq!(
            runnable(odd.into(), true),
            Err(unusable_location()),
            "{odd}"
        );
    }
    assert!(runnable("/home/me/$apps/wick".into(), false).is_ok());
}

#[test]
fn codex_files_wick_created_go_with_its_hooks() {
    let home = Home::new();
    let (config, hooks) = home.codex(None, None);
    connect(&home.places, Agent::Codex).unwrap();
    assert!(config.exists() && hooks.exists());
    disconnect(&home.places, Agent::Codex).unwrap();
    assert!(!config.exists() && !hooks.exists());
    assert!(!backup(&config).exists() && !backup(&hooks).exists());
}

#[test]
fn a_leftover_trust_entry_does_not_carry_over_to_a_new_hook() {
    let home = Home::new();
    let (config, hooks) = home.codex(None, Some(CODEX_HOOKS));
    let key = hooks.display().to_string();
    fs::write(
        &config,
        format!("[hooks.state.'{key}:stop:1:0']\ntrusted_hash = \"sha256:old\"\nenabled = false\n"),
    )
    .unwrap();
    connect(&home.places, Agent::Codex).unwrap();
    let document: DocumentMut = read(&config).parse().unwrap();
    let entry = document["hooks"]["state"][&format!("{key}:stop:1:0")]
        .as_table_like()
        .unwrap();
    assert!(entry.get("enabled").is_none());
    assert_eq!(home.status(Agent::Codex).status, Status::Connected);
}

#[test]
fn codex_hooks_kept_in_config_toml_stay_there() {
    let home = Home::new();
    let original = "model = \"gpt-6.1-sol\"\n\n[[hooks.Stop]]\n[[hooks.Stop.hooks]]\ntype = \"command\"\ncommand = \"notify-send done\"\n";
    let (config, hooks) = home.codex(Some(original), None);
    let key = config.display().to_string();
    assert_eq!(home.status(Agent::Codex).file, "~/.codex/config.toml");
    connect(&home.places, Agent::Codex).unwrap();
    assert!(!hooks.exists());
    let text = read(&config);
    assert!(text.starts_with(original), "{text}");
    let document: DocumentMut = text.parse().unwrap();
    let stop = document["hooks"]["Stop"].as_array_of_tables().unwrap();
    assert_eq!(stop.len(), 2);
    let handlers = stop.get(1).unwrap()["hooks"].as_array_of_tables().unwrap();
    assert_eq!(
        handlers.get(0).unwrap()["command"].as_str(),
        Some(home.command(Agent::Codex).as_str())
    );
    assert!(codex_trust(&config)
        .iter()
        .any(|(name, _)| *name == format!("{key}:stop:1:0")));
    assert_eq!(home.status(Agent::Codex).status, Status::Connected);
    connect(&home.places, Agent::Codex).unwrap();
    assert_eq!(read(&config), text);
    disconnect(&home.places, Agent::Codex).unwrap();
    assert_eq!(read(&config), original);
}

#[test]
fn invalid_codex_files_are_left_alone() {
    let home = Home::new();
    let (config, hooks) = home.codex(Some("model = \n"), None);
    assert_eq!(
        home.status(Agent::Codex).error.as_deref(),
        Some("~/.codex/config.toml isn’t valid TOML. Wick left it unchanged.")
    );
    assert!(connect(&home.places, Agent::Codex).is_err());
    assert_eq!(read(&config), "model = \n");
    assert!(!hooks.exists());
    fs::write(&config, "").unwrap();
    fs::write(&hooks, "{").unwrap();
    assert_eq!(
        connect(&home.places, Agent::Codex).unwrap_err(),
        "~/.codex/hooks.json isn’t valid JSON. Wick left it unchanged."
    );
    assert_eq!(read(&config), "");
    assert_eq!(read(&hooks), "{");
}

#[test]
fn shown_paths_use_a_tilde_under_home() {
    let home = Some(Path::new("/home/me"));
    assert_eq!(
        shown(Path::new("/home/me/.claude/settings.json"), home),
        "~/.claude/settings.json"
    );
    assert_eq!(
        shown(Path::new("/etc/claude/settings.json"), home),
        "/etc/claude/settings.json"
    );
}

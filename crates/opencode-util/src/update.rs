//! FEAT-080: detect a newer published ORT artifact revision.
//!
//! Detection resolves the revision published for the active channel and
//! compares it with the revision the running artifact was installed under. The
//! artifact install itself is delegated to the standards `ort-install` helper
//! so channel resolution and checksum verification have a single owner.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const DEFAULT_REPO: &str = "cchris-p/opencode-modded-rust";
const DEFAULT_INTERVAL_SECS: u64 = 900;
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const PREVIEW_TAG: &str = "native-artifacts";
const BIN_NAME: &str = "opencode-modded-rust";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Preview,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
        }
    }

    pub fn from_env() -> Self {
        match env::var("OPENCODE_RUST_CHANNEL") {
            Ok(value) if value.eq_ignore_ascii_case("preview") => Self::Preview,
            _ => Self::Stable,
        }
    }

    pub fn from_str_lossy(value: &str) -> Self {
        if value.eq_ignore_ascii_case("preview") {
            Self::Preview
        } else {
            Self::Stable
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub channel: Channel,
    pub interval: Duration,
    pub timeout: Duration,
    pub repo: String,
    pub bin_root: PathBuf,
    pub cache_path: PathBuf,
    pub token: Option<String>,
    pub ort_install: Option<PathBuf>,
}

impl UpdateConfig {
    pub fn from_env() -> Self {
        let channel = Channel::from_env();
        let enabled = env_flag("OPENCODE_RUST_UPDATE_CHECK", true);
        let interval_secs = env_u64("OPENCODE_RUST_UPDATE_INTERVAL", DEFAULT_INTERVAL_SECS);
        let cache_path = cache_dir().join("opencode").join("update-check.json");
        Self {
            enabled,
            channel,
            interval: Duration::from_secs(interval_secs),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            repo: env::var("ORT_RELEASE_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string()),
            bin_root: bin_root(),
            cache_path,
            token: github_token(),
            ort_install: ort_install_path(),
        }
    }

    pub fn for_test(cache_path: PathBuf, bin_root: PathBuf) -> Self {
        Self {
            enabled: true,
            channel: Channel::Stable,
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECS),
            timeout: Duration::from_secs(1),
            repo: DEFAULT_REPO.to_string(),
            bin_root,
            cache_path,
            token: None,
            ort_install: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledInfo {
    pub version: String,
    pub rev: Option<String>,
    pub channel: Channel,
    pub bin_root: PathBuf,
    pub artifact_installed: bool,
}

impl InstalledInfo {
    pub fn bin_root(&self) -> &Path {
        &self.bin_root
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub tag: String,
    pub rev: String,
    pub channel: Channel,
    pub dismissed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    Disabled,
    NotArtifact,
    NoUpdate,
    Available(AvailableUpdate),
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallReport {
    pub success: bool,
    pub message: String,
}

impl InstallReport {
    fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}

pub fn installed_info(config: &UpdateConfig) -> InstalledInfo {
    let exe = env::current_exe().ok();
    let rev = exe
        .as_deref()
        .and_then(|path| installed_rev_from_exe(path, &config.bin_root));
    InstalledInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        artifact_installed: rev.is_some(),
        rev,
        channel: config.channel,
        bin_root: config.bin_root.clone(),
    }
}

pub fn installed_rev_from_exe(exe: &Path, bin_root: &Path) -> Option<String> {
    let resolved = fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    let rev_dir = resolved.parent()?;
    let bin_dir = rev_dir.parent()?;
    if bin_dir.file_name()?.to_str()? != "bin" {
        return None;
    }
    if bin_dir.parent()? != bin_root {
        return None;
    }
    if resolved.file_name()?.to_str()? != BIN_NAME {
        return None;
    }
    let rev = rev_dir.file_name()?.to_str()?.trim();
    if rev.is_empty() || rev == "current" {
        return None;
    }
    Some(rev.to_string())
}

pub fn short_rev(rev: &str) -> &str {
    let end = rev
        .char_indices()
        .nth(7)
        .map(|(idx, _)| idx)
        .unwrap_or(rev.len());
    &rev[..end]
}

pub fn check(config: &UpdateConfig) -> CheckOutcome {
    check_impl(config, now_unix(), false)
}

pub fn check_now(config: &UpdateConfig) -> CheckOutcome {
    check_impl(config, now_unix(), true)
}

pub fn check_at(config: &UpdateConfig, now: i64) -> CheckOutcome {
    check_impl(config, now, false)
}

fn check_impl(config: &UpdateConfig, now: i64, force: bool) -> CheckOutcome {
    if !config.enabled {
        return CheckOutcome::Disabled;
    }
    let installed = installed_info(config);
    let Some(installed_rev) = installed.rev.as_deref() else {
        return CheckOutcome::NotArtifact;
    };

    let mut cache = Cache::load(&config.cache_path);
    if !force
        && cache.channel == config.channel.as_str()
        && is_throttled(cache.last_attempt, now, config.interval)
    {
        return cached_outcome(&cache, installed_rev, config.channel);
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(config.timeout)
        .user_agent(format!("opencode-modded-rust/{}", installed.version))
        .build()
    {
        Ok(client) => client,
        Err(_) => return CheckOutcome::Unknown,
    };

    let resolved = resolve_published(&client, config)
        .ok()
        .and_then(|published| {
            fetch_published_rev(&client, config, &published.tag)
                .ok()
                .map(|rev| (published, rev))
        });

    cache.last_attempt = now;
    cache.channel = config.channel.as_str().to_string();
    match resolved {
        Some((published, rev)) => {
            cache.tag = Some(published.tag.clone());
            cache.version = Some(published.version.clone());
            cache.rev = Some(rev.clone());
            let outcome = decide(
                installed_rev,
                &installed.version,
                &published,
                &rev,
                config.channel,
                cache.dismissed_rev.as_deref(),
            );
            let _ = cache.save(&config.cache_path);
            outcome
        }
        None => {
            let _ = cache.save(&config.cache_path);
            CheckOutcome::Unknown
        }
    }
}

pub fn dismiss(config: &UpdateConfig, rev: &str) {
    let mut cache = Cache::load(&config.cache_path);
    cache.dismissed_rev = Some(rev.to_string());
    let _ = cache.save(&config.cache_path);
}

pub fn install(config: &UpdateConfig, update: &AvailableUpdate) -> InstallReport {
    let Some(helper) = config.ort_install.as_deref() else {
        return InstallReport::failure(
            "ort-install was not found; install the standards helper or run it manually.",
        );
    };
    let mut command = std::process::Command::new(helper);
    command
        .arg("--channel")
        .arg(update.channel.as_str())
        .arg("--quiet")
        .env("ORT_BIN_ROOT", &config.bin_root);
    let output = match command.output() {
        Ok(output) => output,
        Err(err) => {
            return InstallReport::failure(format!("failed to run ort-install: {err}"));
        }
    };
    if output.status.success() {
        InstallReport {
            success: true,
            message: format!(
                "Installed {} ({}). Restart the session to apply.",
                update.version,
                short_rev(&update.rev)
            ),
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        let message = if message.is_empty() {
            format!("ort-install exited with {}", output.status)
        } else {
            message.to_string()
        };
        InstallReport::failure(message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Published {
    tag: String,
    version: String,
}

fn resolve_published(
    client: &reqwest::blocking::Client,
    config: &UpdateConfig,
) -> Result<Published, ()> {
    match config.channel {
        Channel::Preview => Ok(Published {
            tag: PREVIEW_TAG.to_string(),
            version: "preview".to_string(),
        }),
        Channel::Stable => {
            let url = format!(
                "https://api.github.com/repos/{}/releases/latest",
                config.repo
            );
            let body = get_json(client, config, &url)?;
            let tag = parse_latest_tag(&body)?;
            if tag == PREVIEW_TAG {
                return Err(());
            }
            Ok(Published {
                version: tag.clone(),
                tag,
            })
        }
    }
}

fn fetch_published_rev(
    client: &reqwest::blocking::Client,
    config: &UpdateConfig,
    tag: &str,
) -> Result<String, ()> {
    let asset = asset_name();
    let release_url = format!(
        "https://api.github.com/repos/{}/releases/tags/{}",
        config.repo, tag
    );
    let body = get_json(client, config, &release_url)?;
    let asset_url = parse_asset_url(&body, &format!("{asset}.rev"));

    let text = match asset_url {
        Some(url) => get_text(client, config, &url, "application/octet-stream")?,
        None => {
            let url = format!(
                "https://github.com/{}/releases/download/{}/{}.rev",
                config.repo, tag, asset
            );
            get_text(client, config, &url, "text/plain")?
        }
    };
    parse_rev(&text).ok_or(())
}

fn get_json(
    client: &reqwest::blocking::Client,
    config: &UpdateConfig,
    url: &str,
) -> Result<String, ()> {
    get_text(client, config, url, "application/vnd.github+json")
}

fn get_text(
    client: &reqwest::blocking::Client,
    config: &UpdateConfig,
    url: &str,
    accept: &str,
) -> Result<String, ()> {
    let mut request = client
        .get(url)
        .header(reqwest::header::ACCEPT, accept)
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = config.token.as_deref() {
        request = request.bearer_auth(token);
    }
    let response = request.send().map_err(|_| ())?;
    if !response.status().is_success() {
        return Err(());
    }
    response.text().map_err(|_| ())
}

fn decide(
    installed_rev: &str,
    installed_version: &str,
    published: &Published,
    published_rev: &str,
    channel: Channel,
    dismissed_rev: Option<&str>,
) -> CheckOutcome {
    if published_rev == installed_rev {
        return CheckOutcome::NoUpdate;
    }
    if let (Some(remote), Some(local)) = (
        parse_version(&published.version),
        parse_version(installed_version),
    ) {
        if remote < local {
            return CheckOutcome::NoUpdate;
        }
    }
    CheckOutcome::Available(AvailableUpdate {
        version: published.version.clone(),
        tag: published.tag.clone(),
        rev: published_rev.to_string(),
        channel,
        dismissed: dismissed_rev == Some(published_rev),
    })
}

fn cached_outcome(cache: &Cache, installed_rev: &str, channel: Channel) -> CheckOutcome {
    let (Some(tag), Some(version), Some(rev)) = (
        cache.tag.as_deref(),
        cache.version.as_deref(),
        cache.rev.as_deref(),
    ) else {
        return CheckOutcome::NoUpdate;
    };
    if rev == installed_rev {
        return CheckOutcome::NoUpdate;
    }
    CheckOutcome::Available(AvailableUpdate {
        version: version.to_string(),
        tag: tag.to_string(),
        rev: rev.to_string(),
        channel,
        dismissed: cache.dismissed_rev.as_deref() == Some(rev),
    })
}

fn is_throttled(last_attempt: i64, now: i64, interval: Duration) -> bool {
    let interval = interval.as_secs() as i64;
    if interval <= 0 {
        return false;
    }
    now.saturating_sub(last_attempt) < interval
}

fn parse_latest_tag(body: &str) -> Result<String, ()> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| ())?;
    let tag = value
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .ok_or(())?;
    Ok(tag.to_string())
}

fn parse_asset_url(body: &str, name: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("assets")?
        .as_array()?
        .iter()
        .find(|asset| asset.get("name").and_then(|n| n.as_str()) == Some(name))
        .and_then(|asset| asset.get("url").and_then(|url| url.as_str()))
        .map(str::to_string)
}

fn parse_rev(text: &str) -> Option<String> {
    let rev = text.lines().next()?.trim();
    if rev.is_empty() {
        return None;
    }
    if !rev
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return None;
    }
    Some(rev.to_string())
}

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let value = value.trim().trim_start_matches(['v', 'V']);
    let core = value.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn asset_name_for(os: &str, arch: &str) -> Option<String> {
    let os = match os {
        "linux" => "linux",
        "macos" => "darwin",
        _ => return None,
    };
    let arch = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some(format!("{BIN_NAME}-{os}-{arch}.tar.gz"))
}

fn asset_name() -> String {
    asset_name_for(env::consts::OS, env::consts::ARCH).unwrap_or_else(|| {
        format!(
            "{BIN_NAME}-{}-{}.tar.gz",
            env::consts::OS,
            env::consts::ARCH
        )
    })
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    #[serde(default)]
    last_attempt: i64,
    #[serde(default)]
    channel: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    rev: Option<String>,
    #[serde(default)]
    dismissed_rev: Option<String>,
}

impl Cache {
    fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) -> std::io::Result<()> {
        let Some(parent) = path.parent() else {
            return Ok(());
        };
        fs::create_dir_all(parent)?;
        let serialized = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);
        fs::rename(tmp, path)
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn env_flag(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        ),
        Err(_) => default,
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    match env::var(name) {
        Ok(value) => value.trim().parse().unwrap_or(default),
        Err(_) => default,
    }
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn data_home() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".local").join("share"))
}

fn cache_dir() -> PathBuf {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".cache"))
}

fn bin_root() -> PathBuf {
    env::var_os("ORT_BIN_ROOT")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| data_home().join("ort"))
}

fn github_token() -> Option<String> {
    for name in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(value) = env::var(name) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    let path = home_dir().join(".config").join("opencode").join(".env");
    let text = fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let value = line.strip_prefix("GH_TOKEN=")?.trim();
        (!value.is_empty()).then(|| value.trim_matches('"').to_string())
    })
}

fn ort_install_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("ORT_INSTALL") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let standard = home_dir()
        .join("standards")
        .join("utilities")
        .join("ort-install");
    if standard.is_file() {
        return Some(standard);
    }
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join("ort-install"))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "opencode-update-test-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn published(tag: &str, version: &str) -> Published {
        Published {
            tag: tag.to_string(),
            version: version.to_string(),
        }
    }

    #[test]
    fn asset_name_matches_platform_naming() {
        assert_eq!(
            asset_name_for("linux", "x86_64").as_deref(),
            Some("opencode-modded-rust-linux-x86_64.tar.gz")
        );
        assert_eq!(
            asset_name_for("macos", "aarch64").as_deref(),
            Some("opencode-modded-rust-darwin-aarch64.tar.gz")
        );
        assert!(asset_name_for("windows", "x86_64").is_none());
    }

    #[test]
    fn parse_rev_accepts_sha_and_rejects_junk() {
        assert_eq!(parse_rev("cc9befa1\n").as_deref(), Some("cc9befa1"));
        assert_eq!(parse_rev("  v0.1.5  ").as_deref(), Some("v0.1.5"));
        assert!(parse_rev("").is_none());
        assert!(parse_rev("abc$def").is_none());
    }

    #[test]
    fn decide_ignores_matching_rev_and_detects_new_rev() {
        let out = decide(
            "aaaa1111",
            "0.1.4",
            &published("v0.1.4", "v0.1.4"),
            "aaaa1111",
            Channel::Stable,
            None,
        );
        assert_eq!(out, CheckOutcome::NoUpdate);

        let out = decide(
            "aaaa1111",
            "0.1.4",
            &published("v0.1.5", "v0.1.5"),
            "bbbb2222",
            Channel::Stable,
            None,
        );
        match out {
            CheckOutcome::Available(update) => {
                assert_eq!(update.version, "v0.1.5");
                assert_eq!(update.rev, "bbbb2222");
                assert!(!update.dismissed);
            }
            other => panic!("expected update, got {other:?}"),
        }
    }

    #[test]
    fn decide_suppresses_downgrades_and_marks_dismissed() {
        let older = decide(
            "bbbb2222",
            "0.1.5",
            &published("v0.1.4", "v0.1.4"),
            "aaaa1111",
            Channel::Stable,
            None,
        );
        assert_eq!(older, CheckOutcome::NoUpdate);

        let dismissed = decide(
            "aaaa1111",
            "0.1.4",
            &published("v0.1.5", "v0.1.5"),
            "bbbb2222",
            Channel::Stable,
            Some("bbbb2222"),
        );
        match dismissed {
            CheckOutcome::Available(update) => assert!(update.dismissed),
            other => panic!("expected update, got {other:?}"),
        }
    }

    #[test]
    fn throttle_respects_interval() {
        let interval = Duration::from_secs(900);
        assert!(is_throttled(1_000, 1_500, interval));
        assert!(!is_throttled(1_000, 1_900, interval));
        assert!(!is_throttled(1_000, 1_001, Duration::ZERO));
    }

    #[test]
    fn installed_rev_requires_artifact_bin_layout() {
        let root = temp_dir("layout");
        let rev_dir = root.join("bin").join("cc9befa1");
        fs::create_dir_all(&rev_dir).unwrap();
        let exe = rev_dir.join(BIN_NAME);
        fs::write(&exe, b"").unwrap();
        assert_eq!(
            installed_rev_from_exe(&exe, &root).as_deref(),
            Some("cc9befa1")
        );

        let dev = root.join("target").join("debug").join(BIN_NAME);
        fs::create_dir_all(dev.parent().unwrap()).unwrap();
        fs::write(&dev, b"").unwrap();
        assert!(installed_rev_from_exe(&dev, &root).is_none());
    }

    #[test]
    fn cache_roundtrip_and_cached_outcome() {
        let root = temp_dir("cache");
        let path = root.join("nested").join("update-check.json");
        let cache = Cache {
            last_attempt: 42,
            channel: "stable".to_string(),
            tag: Some("v0.1.5".to_string()),
            version: Some("v0.1.5".to_string()),
            rev: Some("bbbb2222".to_string()),
            dismissed_rev: Some("bbbb2222".to_string()),
        };
        cache.save(&path).unwrap();
        let loaded = Cache::load(&path);
        assert_eq!(loaded.last_attempt, 42);
        assert_eq!(loaded.rev.as_deref(), Some("bbbb2222"));

        let outcome = cached_outcome(&loaded, "aaaa1111", Channel::Stable);
        match outcome {
            CheckOutcome::Available(update) => {
                assert_eq!(update.rev, "bbbb2222");
                assert!(update.dismissed);
            }
            other => panic!("expected cached update, got {other:?}"),
        }
        assert_eq!(
            cached_outcome(&loaded, "bbbb2222", Channel::Stable),
            CheckOutcome::NoUpdate
        );
    }

    #[test]
    fn disabled_config_never_checks() {
        let root = temp_dir("disabled");
        let mut config = UpdateConfig::for_test(root.join("cache.json"), root);
        config.enabled = false;
        assert_eq!(check_at(&config, 100), CheckOutcome::Disabled);
    }

    #[test]
    fn version_parsing_orders_stable_tags() {
        assert!(parse_version("v0.1.5") > parse_version("0.1.4"));
        assert_eq!(parse_version("v1"), Some((1, 0, 0)));
        assert_eq!(parse_version("junk"), None);
    }

    #[test]
    fn short_rev_is_seven_chars() {
        assert_eq!(short_rev("cc9befa1"), "cc9befa");
        assert_eq!(short_rev("abc"), "abc");
    }
}

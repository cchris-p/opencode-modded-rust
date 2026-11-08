use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;
use tempfile::TempDir;

use crate::config::Config;
use crate::engine::{ChatMessage, Engine};
use crate::Runtime;

struct TestEnv {
    _temp: TempDir,
    pub repo: PathBuf,
    pub runtime: Runtime,
}

impl TestEnv {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        let home = temp.path().join("home");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&home).unwrap();
        git(&repo, &["init", "-q"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        git(&repo, &["config", "user.name", "Test User"]);

        let mut env = HashMap::new();
        env.insert("OCO_AI_PROVIDER".to_string(), "test".to_string());
        env.insert(
            "OCO_TEST_MOCK_TYPE".to_string(),
            "commit-message".to_string(),
        );
        env.insert("OCO_API_KEY".to_string(), "test-key".to_string());

        let runtime = Runtime {
            cwd: repo.clone(),
            home: home.clone(),
            global_config_path: home.join(".opencommit"),
            models_cache_path: home.join(".cache/ort/oco-models.json"),
            env,
            interactive: false,
        };

        Self {
            _temp: temp,
            repo,
            runtime,
        }
    }

    fn set_env(&mut self, key: &str, value: &str) {
        self.runtime.env.insert(key.to_string(), value.to_string());
    }

    fn stage(&self, name: &str, contents: &str) {
        fs::write(self.repo.join(name), contents).unwrap();
        git(&self.repo, &["add", name]);
    }

    fn log_messages(&self) -> Vec<String> {
        let output = git_output(&self.repo, &["log", "--format=%B%x00"]);
        output
            .split('\0')
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .map(ToString::to_string)
            .collect()
    }
}

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).to_string()
}

struct MockServer {
    base_url: String,
    captured: Arc<Mutex<Vec<String>>>,
    headers: Arc<Mutex<Vec<String>>>,
}

impl MockServer {
    fn spawn(response_body: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_thread = captured.clone();
        let headers = Arc::new(Mutex::new(Vec::new()));
        let headers_thread = headers.clone();
        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0usize;
                let mut request_lines = Vec::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    let trimmed = line.trim_end().to_string();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(value) =
                        trimmed.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                    request_lines.push(trimmed);
                }
                headers_thread.lock().unwrap().extend(request_lines.clone());
                let mut body = vec![0u8; content_length];
                let _ = reader.read_exact(&mut body);
                let body = String::from_utf8_lossy(&body).to_string();
                captured_thread.lock().unwrap().push(body);

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let mut stream = stream;
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            base_url: format!("http://{address}"),
            captured,
            headers,
        }
    }

    fn last_request(&self) -> String {
        self.captured
            .lock()
            .unwrap()
            .last()
            .cloned()
            .unwrap_or_default()
    }

    fn sent_authorization(&self) -> bool {
        self.headers.lock().unwrap().iter().any(|line| {
            line.to_ascii_lowercase()
                .starts_with("authorization: bearer ")
        })
    }
}

fn provider_config(provider: &str, base_url: &str) -> Config {
    let mut config = Config::defaults();
    config.set("OCO_AI_PROVIDER", Value::String(provider.to_string()));
    config.set("OCO_API_KEY", Value::String("test-key".to_string()));
    config.set("OCO_API_URL", Value::String(base_url.to_string()));
    config.set(
        "OCO_MODEL",
        Value::String(crate::config::default_model(provider)),
    );
    config
}

#[test]
fn config_precedence_is_env_then_dotenv_then_global_then_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    let global = temp.path().join(".opencommit");
    fs::write(
        &global,
        "OCO_AI_PROVIDER=anthropic\nOCO_MODEL=global-model\nOCO_EMOJI=true\n",
    )
    .unwrap();
    fs::write(
        repo.join(".env"),
        "OCO_MODEL=env-model\nOCO_DESCRIPTION=true\n",
    )
    .unwrap();
    let mut env = HashMap::new();
    env.insert("OCO_MODEL".to_string(), "process-model".to_string());

    let config = Config::load(&global, &repo, &env).unwrap();
    assert_eq!(config.string("OCO_MODEL").unwrap(), "process-model");
    assert!(config.bool_or("OCO_DESCRIPTION", false));
    assert!(config.bool_or("OCO_EMOJI", false));
    assert_eq!(config.provider(), "anthropic");
    assert_eq!(config.u64_or("OCO_TOKENS_MAX_INPUT", 0), 4096);
}

#[test]
fn empty_url_and_headers_are_valid_cleared_values() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".opencommit");
    fs::write(
        &path,
        "OCO_AI_PROVIDER=openai\nOCO_MODEL=gpt-4o\nOCO_API_URL=\nOCO_API_CUSTOM_HEADERS=\nOCO_API_KEY=test\n",
    )
    .unwrap();
    let config = Config::load(&path, temp.path(), &HashMap::new()).unwrap();
    assert_eq!(config.string("OCO_API_URL").unwrap(), "");
}

#[test]
fn config_round_trips_quoted_values() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".opencommit");
    let mut config = Config::defaults();
    config.set(
        "OCO_API_CUSTOM_HEADERS",
        Value::String("{\"X-Test\":\"a b\"}".to_string()),
    );
    config.write(&path).unwrap();
    let loaded = Config::load(&path, temp.path(), &HashMap::new()).unwrap();
    assert_eq!(
        loaded.string("OCO_API_CUSTOM_HEADERS").unwrap(),
        "{\"X-Test\":\"a b\"}"
    );
}

#[tokio::test]
async fn commit_flow_creates_exactly_one_commit() {
    let env = TestEnv::new();
    env.stage("file.txt", "hello\n");

    env.runtime.run(vec!["--yes".to_string()]).await.unwrap();
    let messages = env.log_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "fix(testAi.ts): test commit message");
}

#[tokio::test]
async fn message_template_replaces_placeholder_and_is_not_forwarded() {
    let env = TestEnv::new();
    env.stage("file.txt", "hello\n");

    env.runtime
        .run(vec!["--yes".to_string(), "#205: $msg".to_string()])
        .await
        .unwrap();
    let messages = env.log_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "#205: fix(testAi.ts): test commit message");
    let show = git_output(&env.repo, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(!show.contains("$msg"));
}

#[tokio::test]
async fn no_verify_is_forwarded_to_git_commit() {
    let env = TestEnv::new();
    let hooks = env.repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-commit");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&hook).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).unwrap();
    }

    env.stage("file.txt", "hello\n");
    let without = env.runtime.run(vec!["--yes".to_string()]).await;
    assert!(without.is_err());

    let with = env
        .runtime
        .run(vec!["--yes".to_string(), "--no-verify".to_string()])
        .await;
    assert!(with.is_ok());
    assert_eq!(env.log_messages().len(), 1);
}

#[tokio::test]
async fn context_is_included_in_the_provider_prompt() {
    let server = MockServer::spawn(
        "{\"choices\":[{\"message\":{\"content\":\"feat: mock message\"}}]}".to_string(),
    );
    let mut env = TestEnv::new();
    env.set_env("OCO_AI_PROVIDER", "openai");
    env.set_env("OCO_API_URL", &format!("{}/v1", server.base_url));
    env.set_env("OCO_API_KEY", "test-key");
    env.stage("file.txt", "hello\n");

    env.runtime
        .run(vec![
            "--yes".to_string(),
            "-c".to_string(),
            "context-marker".to_string(),
        ])
        .await
        .unwrap();

    let request = server.last_request();
    assert!(request.contains("context-marker"));
    assert!(request.contains("git diff --staged"));
    assert!(request.contains("file.txt"));
    assert_eq!(env.log_messages()[0], "feat: mock message");
}

#[tokio::test]
async fn openai_adapter_sends_chat_completions_request() {
    let server = MockServer::spawn(
        "{\"choices\":[{\"message\":{\"content\":\"feat: openai\"}}]}".to_string(),
    );
    let config = provider_config("openai", &format!("{}/v1", server.base_url));
    let engine = Engine::from_config(&config).unwrap();
    let message = engine
        .generate(&[ChatMessage::user("hello")])
        .await
        .unwrap();
    assert_eq!(message, "feat: openai");
    assert!(server.last_request().contains("\"model\":\"gpt-4o-mini\""));
    assert!(!server.last_request().contains("\"stream\":false"));
    assert!(server.sent_authorization());
}

#[tokio::test]
async fn anthropic_adapter_sends_messages_request() {
    let server = MockServer::spawn(
        "{\"content\":[{\"type\":\"text\",\"text\":\"feat: anthropic\"}]}".to_string(),
    );
    let config = provider_config("anthropic", &format!("{}/v1", server.base_url));
    let engine = Engine::from_config(&config).unwrap();
    let message = engine
        .generate(&[ChatMessage::user("hello")])
        .await
        .unwrap();
    assert_eq!(message, "feat: anthropic");
    assert!(server.last_request().contains("hello"));
}

#[tokio::test]
async fn gemini_adapter_sends_generate_content_request() {
    let server = MockServer::spawn(
        "{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"feat: gemini\"}]}}]}".to_string(),
    );
    let config = provider_config("gemini", &format!("{}/v1beta", server.base_url));
    let engine = Engine::from_config(&config).unwrap();
    let message = engine
        .generate(&[ChatMessage::user("hello")])
        .await
        .unwrap();
    assert_eq!(message, "feat: gemini");
    assert!(server.last_request().contains("\"contents\""));
}

#[tokio::test]
async fn ollama_adapter_uses_native_api_chat() {
    let server = MockServer::spawn("{\"message\":{\"content\":\"feat: ollama\"}}".to_string());
    let config = provider_config("ollama", &server.base_url);
    let engine = Engine::from_config(&config).unwrap();
    let message = engine
        .generate(&[ChatMessage::user("hello")])
        .await
        .unwrap();
    assert_eq!(message, "feat: ollama");
    assert!(server.last_request().contains("\"options\""));
}

#[tokio::test]
async fn unreachable_provider_fails_with_actionable_error() {
    let mut config = Config::defaults();
    config.set("OCO_AI_PROVIDER", Value::String("openai".to_string()));
    config.set("OCO_API_KEY", Value::String("test-key".to_string()));
    config.set(
        "OCO_API_URL",
        Value::String("http://127.0.0.1:1".to_string()),
    );
    let engine = Engine::from_config(&config).unwrap();
    let error = engine
        .generate(&[ChatMessage::user("hello")])
        .await
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("openai"));
    assert!(message.contains("127.0.0.1"));
}

#[tokio::test]
async fn hook_set_run_and_unset_round_trip() {
    let env = TestEnv::new();
    env.stage("file.txt", "hello\n");

    env.runtime
        .run(vec!["hook".to_string(), "set".to_string()])
        .await
        .unwrap();
    let hook = env.repo.join(".git/hooks/prepare-commit-msg");
    assert!(hook.exists());
    assert!(fs::read_to_string(&hook)
        .unwrap()
        .contains("opencommit-managed"));

    let message_file = env.repo.join("COMMIT_EDITMSG");
    fs::write(&message_file, "original message\n").unwrap();
    env.runtime
        .run(vec![
            "hook".to_string(),
            "run".to_string(),
            message_file.display().to_string(),
        ])
        .await
        .unwrap();
    let written = fs::read_to_string(&message_file).unwrap();
    assert!(written.contains("# fix(testAi.ts): test commit message"));
    assert!(written.contains("[OpenCommit]"));
    assert!(written.contains("original message"));

    env.runtime
        .run(vec!["hook".to_string(), "unset".to_string()])
        .await
        .unwrap();
    assert!(!hook.exists());
}

#[tokio::test]
async fn hook_set_preserves_a_different_existing_hook() {
    let env = TestEnv::new();
    let hooks = env.repo.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("prepare-commit-msg");
    let custom = "#!/bin/sh\necho custom\n";
    fs::write(&hook, custom).unwrap();

    let result = env
        .runtime
        .run(vec!["hook".to_string(), "set".to_string()])
        .await;
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&hook).unwrap(), custom);
}

#[tokio::test]
async fn hook_set_honors_core_hooks_path() {
    let env = TestEnv::new();
    git(&env.repo, &["config", "core.hooksPath", ".githooks"]);

    env.runtime
        .run(vec!["hook".to_string(), "set".to_string()])
        .await
        .unwrap();
    assert!(env.repo.join(".githooks/prepare-commit-msg").exists());
}

#[tokio::test]
async fn hook_auto_uncomment_writes_message_without_comments() {
    let mut env = TestEnv::new();
    env.set_env("OCO_HOOK_AUTO_UNCOMMENT", "true");
    env.stage("file.txt", "hello\n");

    let message_file = env.repo.join("COMMIT_EDITMSG");
    fs::write(&message_file, "original message\n").unwrap();
    env.runtime
        .run(vec![
            "hook".to_string(),
            "run".to_string(),
            message_file.display().to_string(),
        ])
        .await
        .unwrap();

    let written = fs::read_to_string(&message_file).unwrap();
    assert!(written.starts_with("fix(testAi.ts): test commit message"));
    assert!(!written.contains("[OpenCommit]"));
}

#[tokio::test]
async fn commitlint_force_and_get_round_trip() {
    let mut env = TestEnv::new();
    env.set_env("OCO_TEST_MOCK_TYPE", "prompt-module-commitlint-config");
    fs::write(
        env.repo.join(".commitlintrc.json"),
        r#"{
          "rules": {
            "type-enum": [2, "always", ["feat", "fix"]],
            "subject-empty": [2, "never"]
          }
        }"#,
    )
    .unwrap();

    env.runtime
        .run(vec!["commitlint".to_string(), "force".to_string()])
        .await
        .unwrap();
    let config = crate::commitlint::read_llm_config(&env.runtime).unwrap();
    assert_eq!(config.prompts.len(), 2);
    assert!(config
        .consistency
        .get("english")
        .unwrap()
        .get("commitFix")
        .is_some());

    env.runtime
        .run(vec!["commitlint".to_string(), "get".to_string()])
        .await
        .unwrap();
}

#[tokio::test]
async fn commitlint_prompt_module_generates_a_commit() {
    let mut env = TestEnv::new();
    env.set_env("OCO_TEST_MOCK_TYPE", "prompt-module-commitlint-config");
    env.set_env("OCO_PROMPT_MODULE", "@commitlint");
    fs::write(
        env.repo.join(".commitlintrc.json"),
        r#"{"rules":{"type-enum":[2,"always",["feat","fix"]]}}"#,
    )
    .unwrap();
    env.stage("file.txt", "hello\n");

    env.runtime.run(vec!["--yes".to_string()]).await.unwrap();
    assert_eq!(env.log_messages().len(), 1);
}

#[tokio::test]
async fn models_command_caches_ollama_models() {
    let server = MockServer::spawn(
        "{\"models\":[{\"name\":\"qwen2.5-coder:7b\"},{\"name\":\"llama3:8b\"}]}".to_string(),
    );
    let mut env = TestEnv::new();
    env.set_env("OCO_AI_PROVIDER", "ollama");
    env.set_env("OCO_API_URL", &server.base_url);

    env.runtime
        .run(vec![
            "models".to_string(),
            "--provider".to_string(),
            "ollama".to_string(),
            "--refresh".to_string(),
        ])
        .await
        .unwrap();

    let cache = fs::read_to_string(&env.runtime.models_cache_path).unwrap();
    let value: Value = serde_json::from_str(&cache).unwrap();
    let models = value["models"]["ollama"].as_array().unwrap();
    assert!(models.iter().any(|model| model == "qwen2.5-coder:7b"));
}

#[tokio::test]
async fn config_command_set_persists_global_values() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let repo = temp.path().join("repo");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&repo).unwrap();
    let runtime = Runtime {
        cwd: repo.clone(),
        home: home.clone(),
        global_config_path: home.join(".opencommit"),
        models_cache_path: home.join(".cache/ort/oco-models.json"),
        env: HashMap::new(),
        interactive: false,
    };

    runtime
        .run(vec![
            "config".to_string(),
            "set".to_string(),
            "OCO_MODEL=gpt-4o".to_string(),
        ])
        .await
        .unwrap();

    let config = Config::load(&runtime.global_config_path, &repo, &HashMap::new()).unwrap();
    assert_eq!(config.string("OCO_MODEL").unwrap(), "gpt-4o");
}

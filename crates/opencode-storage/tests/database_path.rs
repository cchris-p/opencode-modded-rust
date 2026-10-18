use std::path::PathBuf;
use std::sync::Mutex;

use opencode_storage::Database;

const ENV_KEY: &str = "OPENCODE_DATA_DIR";

/// `OPENCODE_DATA_DIR` is process-global, so the path tests must not overlap.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_env_lock<F: FnOnce()>(f: F) {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f();
}

fn temp_dir(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "opencode-data-dir-{}-{}",
        label,
        std::process::id()
    ));
    path
}

fn default_database_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("opencode")
        .join("opencode.db")
}

#[test]
fn opencode_data_dir_takes_precedence() {
    with_env_lock(|| {
        let data_dir = temp_dir("precedence");
        std::env::set_var(ENV_KEY, &data_dir);

        let path = Database::get_database_path().expect("resolve database path");

        assert_eq!(path, data_dir.join("opencode.db"));

        std::env::remove_var(ENV_KEY);
    });
}

#[test]
fn unset_opencode_data_dir_falls_back_to_default() {
    with_env_lock(|| {
        std::env::remove_var(ENV_KEY);

        let path = Database::get_database_path().expect("resolve database path");
        assert_eq!(path, default_database_path());

        std::env::set_var(ENV_KEY, "   ");
        let blank_path = Database::get_database_path().expect("resolve database path");
        assert_eq!(blank_path, default_database_path());

        std::env::remove_var(ENV_KEY);
    });
}

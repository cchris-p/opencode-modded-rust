use crate::config::Config;
use crate::engine::ChatMessage;

pub const IDENTITY: &str = "You are to act as an author of a commit message in git.";

const GITMOJI_HELP: &str = "Use GitMoji convention to preface the commit. Here are some help to choose the right emoji (emoji, description): \
🐛, Fix a bug; \
✨, Introduce new features; \
📝, Add or update documentation; \
🚀, Deploy stuff; \
✅, Add, update, or pass tests; \
♻️, Refactor code; \
⬆️, Upgrade dependencies; \
🔧, Add or update configuration files; \
🌐, Internationalization and localization; \
💡, Add or update comments in source code;";

const FULL_GITMOJI_SPEC: &str = "Use GitMoji convention to preface the commit. Here are some help to choose the right emoji (emoji, description): \
🐛, Fix a bug; \
✨, Introduce new features; \
📝, Add or update documentation; \
🚀, Deploy stuff; \
✅, Add, update, or pass tests; \
♻️, Refactor code; \
⬆️, Upgrade dependencies; \
🔧, Add or update configuration files; \
🌐, Internationalization and localization; \
💡, Add or update comments in source code; \
🎨, Improve structure / format of the code; \
⚡️, Improve performance; \
🔥, Remove code or files; \
🚑️, Critical hotfix; \
💄, Add or update the UI and style files; \
🎉, Begin a project; \
🔒️, Fix security issues; \
🔐, Add or update secrets; \
🔖, Release / Version tags; \
🚨, Fix compiler / linter warnings; \
🚧, Work in progress; \
💚, Fix CI Build; \
⬇️, Downgrade dependencies; \
📌, Pin dependencies to specific versions; \
👷, Add or update CI build system; \
📈, Add or update analytics or track code; \
➕, Add a dependency; \
➖, Remove a dependency; \
🔨, Add or update development scripts; \
✏️, Fix typos; \
💩, Write bad code that needs to be improved; \
⏪️, Revert changes; \
🔀, Merge branches; \
📦️, Add or update compiled files or packages; \
👽️, Update code due to external API changes; \
🚚, Move or rename resources (e.g.: files, paths, routes); \
📄, Add or update license; \
💥, Introduce breaking changes; \
🍱, Add or update assets; \
♿️, Improve accessibility; \
🍻, Write code drunkenly; \
💬, Add or update text and literals; \
🗃️, Perform database related changes; \
🔊, Add or update logs; \
🔇, Remove logs; \
👥, Add or update contributor(s); \
🚸, Improve user experience / usability; \
🏗️, Make architectural changes; \
📱, Work on responsive design; \
🤡, Mock things; \
🥚, Add or update an easter egg; \
🙈, Add or update a .gitignore file; \
📸, Add or update snapshots; \
⚗️, Perform experiments; \
🔍️, Improve SEO; \
🏷️, Add or update types; \
🌱, Add or update seed files; \
🚩, Add, update, or remove feature flags; \
🥅, Catch errors; \
💫, Add or update animations and transitions; \
🗑️, Deprecate code that needs to be cleaned up; \
🛂, Work on code related to authorization, roles and permissions; \
🩹, Simple fix for a non-critical issue; \
🧐, Data exploration/inspection; \
⚰️, Remove dead code; \
🧪, Add a failing test; \
👔, Add or update business logic; \
🩺, Add or update healthcheck; \
🧱, Infrastructure related changes; \
🧑‍💻, Improve developer experience; \
💸, Add sponsorships or money related infrastructure; \
🧵, Add or update code related to multithreading or concurrency; \
🦺, Add or update code related to validation.";

const CONVENTIONAL_COMMIT_KEYWORDS: &str = "Do not preface the commit with anything, except for the conventional commit keywords: fix, feat, build, chore, ci, docs, style, refactor, perf, test.";

pub const INIT_DIFF_PROMPT: &str = "diff --git a/src/server.ts b/src/server.ts
    index ad4db42..f3b18a9 100644
    --- a/src/server.ts
    +++ b/src/server.ts
    @@ -10,7 +10,7 @@
    import {
        initWinstonLogger();
        
        const app = express();
        -const port = 7799;
        +const PORT = 7799;
        
        app.use(express.json());
        
        @@ -34,6 +34,6 @@
        app.use((_, res, next) => {
            // ROUTES
            app.use(PROTECTED_ROUTER_URL, protectedRouter);
            
            -app.listen(port, () => {
                -  console.log(`Server listening on port ${port}`);
                +app.listen(process.env.PORT || PORT, () => {
                    +  console.log(`Server listening on port ${PORT}`);
                });";

fn commit_convention(config: &Config, full_gitmoji_spec: bool) -> &'static str {
    if full_gitmoji_spec {
        FULL_GITMOJI_SPEC
    } else if config.bool_or("OCO_EMOJI", false) {
        GITMOJI_HELP
    } else {
        CONVENTIONAL_COMMIT_KEYWORDS
    }
}

fn description_instruction(config: &Config) -> &'static str {
    if config.bool_or("OCO_DESCRIPTION", false) {
        "Add a short description of WHY the changes are done after the commit message. Don't start it with \"This commit\", just describe the changes."
    } else {
        "Don't add any descriptions to the commit, only commit message."
    }
}

fn one_line_instruction(config: &Config) -> &'static str {
    if config.bool_or("OCO_ONE_LINE_COMMIT", false) {
        "Craft a concise, single sentence, commit message that encapsulates all changes made, with an emphasis on the primary updates. If the modifications share a common theme or scope, mention it succinctly; otherwise, leave the scope out to maintain focus. The goal is to provide a clear and unified overview of the changes in one single message."
    } else {
        ""
    }
}

fn scope_instruction(config: &Config) -> &'static str {
    if config.bool_or("OCO_OMIT_SCOPE", false) {
        "Do not include a scope in the commit message format. Use the format: <type>: <subject>"
    } else {
        ""
    }
}

fn emoji_position_instruction(config: &Config) -> &'static str {
    if config.bool_or("OCO_EMOJI_POSITION_BEFORE_DESCRIPTION", false) {
        "Place the GitMoji emoji at the beginning of the description rather than in the header."
    } else {
        ""
    }
}

fn user_input_context(context: &str) -> String {
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!(
        "Additional context provided by the user: <context>{trimmed}</context>\nConsider this context when generating the commit message, incorporating relevant information when appropriate."
    )
}

pub fn main_prompt(config: &Config, full_gitmoji_spec: bool, context: &str) -> ChatMessage {
    let language = config.language_name();
    let commit_convention = if full_gitmoji_spec {
        "GitMoji specification"
    } else {
        "Conventional Commit Convention"
    };
    let mission_statement = format!(
        "{IDENTITY} Your mission is to create clean and comprehensive commit messages as per the {commit_convention} and explain WHAT were the changes and mainly WHY the changes were done."
    );
    let diff_instruction =
        "I'll send you an output of 'git diff --staged' command, and you are to convert it into a commit message.";
    let general_guidelines = format!(
        "Use the present tense. Lines must not be longer than 74 characters. Use {language} for the commit message."
    );
    let content = format!(
        "{mission_statement}\n{diff_instruction}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        commit_convention_text(config, full_gitmoji_spec),
        description_instruction(config),
        one_line_instruction(config),
        scope_instruction(config),
        emoji_position_instruction(config),
        general_guidelines,
        user_input_context(context),
    );
    ChatMessage::system(content)
}

fn commit_convention_text(config: &Config, full_gitmoji_spec: bool) -> &'static str {
    commit_convention(config, full_gitmoji_spec)
}

/// Initial messages for the conventional-commit prompt module, without the
/// actual diff. Callers append the diff as a final user message.
pub fn build_commit_messages(
    config: &Config,
    full_gitmoji_spec: bool,
    context: &str,
) -> Vec<ChatMessage> {
    vec![
        main_prompt(config, full_gitmoji_spec, context),
        ChatMessage::user(INIT_DIFF_PROMPT),
        consistency_prompt(config),
    ]
}

pub fn consistency_prompt(config: &Config) -> ChatMessage {
    let omit_scope = config.bool_or("OCO_OMIT_SCOPE", false);
    let one_line = config.bool_or("OCO_ONE_LINE_COMMIT", false);

    let fix_message = if omit_scope {
        "fix: change port variable case from lowercase port to uppercase PORT to improve semantics"
    } else {
        "fix(server.ts): change port variable case from lowercase port to uppercase PORT to improve semantics"
    };
    let feat_message = if omit_scope {
        "feat: add support for process.env.PORT environment variable to be able to run app on a configurable port"
    } else {
        "feat(server.ts): add support for process.env.PORT environment variable to be able to run app on a configurable port"
    };

    let fix = generate_commit_string(config, "fix", fix_message);
    let feat = if one_line {
        String::new()
    } else {
        generate_commit_string(config, "feat", feat_message)
    };
    let description = if config.bool_or("OCO_DESCRIPTION", false) {
        "The port variable is now named PORT, which improves consistency with the naming conventions as PORT is a constant. Support for an environment variable allows the application to be more flexible as it can now run on any available port specified via the process.env.PORT environment variable.".to_string()
    } else {
        String::new()
    };

    let content = [fix, feat, description]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    ChatMessage::assistant(content)
}

pub fn generate_commit_string(config: &Config, kind: &str, message: &str) -> String {
    let clean = remove_conventional_commit_word(message);
    if config.bool_or("OCO_EMOJI", false) {
        let emoji = match kind {
            "fix" => "🐛",
            _ => "✨",
        };
        format!("{emoji} {clean}")
    } else {
        message.to_string()
    }
}

pub fn remove_conventional_commit_word(message: &str) -> String {
    for prefix in ["fix", "feat"] {
        let with_scope = format!("{prefix}(");
        if let Some(rest) = message.strip_prefix(&with_scope) {
            if let Some(end) = rest.find("):") {
                return format!("({}):{}", &rest[..end], &rest[end + 2..]);
            }
        }
    }
    message.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn config(pairs: &[(&str, Value)]) -> Config {
        let mut config = Config::defaults();
        for (key, value) in pairs {
            config.set(key, value.clone());
        }
        config
    }

    #[test]
    fn conventional_prompt_uses_keywords_and_language() {
        let config = config(&[("OCO_LANGUAGE", Value::String("de".into()))]);
        let prompt = build_commit_messages(&config, false, "keep it short");
        assert!(prompt[0].content.contains("Conventional Commit Convention"));
        assert!(prompt[0].content.contains("conventional commit keywords"));
        assert!(prompt[0].content.contains("Use German"));
        assert!(prompt[0].content.contains("keep it short"));
        assert!(!prompt[0].content.contains("GitMoji"));
    }

    #[test]
    fn full_gitmoji_prompt_includes_full_set() {
        let config = Config::defaults();
        let prompt = build_commit_messages(&config, true, "");
        assert!(prompt[0].content.contains("GitMoji specification"));
        assert!(prompt[0].content.contains("🦺"));
        assert!(!prompt[2].content.contains("🐛"));
    }

    #[test]
    fn description_and_one_line_are_selected() {
        let config = config(&[
            ("OCO_DESCRIPTION", Value::Bool(true)),
            ("OCO_ONE_LINE_COMMIT", Value::Bool(true)),
        ]);
        let prompt = build_commit_messages(&config, false, "");
        assert!(prompt[0].content.contains("short description"));
        assert!(prompt[0].content.contains("single sentence"));
        assert!(!prompt[2].content.contains("✨"));
        assert!(!prompt[2].content.contains("🐛"));
        assert_eq!(prompt[2].content.lines().count(), 2);
    }

    #[test]
    fn omit_scope_changes_consistency_examples() {
        let config = config(&[("OCO_OMIT_SCOPE", Value::Bool(true))]);
        let prompt = build_commit_messages(&config, false, "");
        assert!(prompt[2].content.contains("fix: change port"));
        assert!(!prompt[2].content.contains("fix(server.ts)"));
    }

    #[test]
    fn emoji_prefaces_consistency_examples() {
        let config = config(&[("OCO_EMOJI", Value::Bool(true))]);
        let prompt = build_commit_messages(&config, false, "");
        assert!(prompt[2].content.contains("🐛 (server.ts):"));
        assert!(prompt[2].content.contains("✨ (server.ts):"));
    }
}

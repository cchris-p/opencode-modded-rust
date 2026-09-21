# Option Selection Invariants

The TUI uses exactly two sanctioned methods for choosing options. Every option surface must use one
of them deliberately; the choice must not collide with text entry.

## The two methods

- **Mnemonic selection** — a single key (a digit or a letter) is bound directly to an option and
  selects or toggles it immediately.
- **Focus selection** — the user moves a highlighted focus with navigation keys (`Up`/`Down`,
  `Left`/`Right`, `Tab`/`Shift+Tab`) and activates the focused option with an activation key
  (`Space`, or `Enter` where submit is not the primary action).

## Binding rules

- A surface that accepts free-text input must use focus selection for its options; it must not bind
  bare printable characters (digits or letters) as option mnemonics.
- Every printable character, including every digit, must be typeable into any text-entry field. A
  character that can legitimately be text must never be captured as a command in that field.
- Mnemonic selection is permitted only in menus or prompts that contain no free-text entry.
- A mnemonic must not collide with any text the same surface accepts. A prompt whose mode can accept
  text uses focus selection even when its sibling choice mode uses mnemonics.
- A text-entry field must be caret-editable: `Left`/`Right` move by one character, `Home`/`End` move
  to the boundaries, typed characters insert at the caret, and `Backspace`/`Delete` remove around the
  caret. Append-only and pop-only editing is a defect.
- The focused option must be visually distinguishable, and the surface's hint text must state the
  real navigation and activation keys.
- Modified or prefixed mnemonics may be used where bare keys would collide, but only when reliable on
  every supported platform. `Alt`+digit is disallowed on macOS because Option+digit emits
  characters.
- A surface must not mix mnemonic and focus selection for the same option set.

## Current application (reference)

- Focus selection:
  - Question prompt navigation and activation: `Up`/`Down` + `Space`/`Enter`
    (`crates/opencode-tui/src/app/app.rs:401-408`).
  - Settings view navigation: arrow keys (`crates/opencode-tui/src/app/app.rs:2698-2712`).
  - General dialog and list navigation.
- Mnemonic selection:
  - Permission prompt letters `y`/`n`/`a`/`p`
    (`crates/opencode-tui/src/app/app.rs:363-386`).
  - Question-prompt choice letters `a`, `b`, `c`, ... by option index
    (`crates/opencode-tui/src/components/question.rs:110-130`, rendered at `:215-223`).
- Known violation: the export dialog binds bare digits `1`/`2`/`3` to its Include-options toggles,
  so a filename or path cannot contain those digits
  (`crates/opencode-tui/src/components/dialogs/session_export.rs:48-55`). This is tracked as
  `BUG-029`.

## Applied decision

- The export dialog's options move to focus selection: `Tab`/`Shift+Tab` cycles focus across the
  filename field and the three option rows, `Space` toggles the focused option, and `Enter` exports.
  Bare digits become typeable in the filename field (`BUG-029`).
- The main prompt input and every dialog text field follow the caret-editability rule via the
  existing cursor model (`crates/opencode-tui/src/components/prompt.rs:75`, `:491-511`).

## Provider Setup

`Settings > Provider` is the authoritative V1 setup path.

Use that screen to:

- choose the active provider
- choose the active model
- review the effective auth state
- review and edit the Ollama host when using local models

Behavior outside that screen is secondary:

- project or global config can still override provider behavior
- environment variables can still override provider behavior
- shell helpers such as local Ollama launch aliases are optional conveniences, not the primary setup flow

The provider screen shows the effective provider, effective model, auth source, and Ollama host source so overrides are visible instead of implicit.

When you press `Enter` on a highlighted model in `Settings > Provider`, the selection is written back to the project config path and becomes the normal default for future runs.

When Ollama is highlighted, press `u` to edit the Ollama host/base URL from the same screen.

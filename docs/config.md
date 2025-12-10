# Kat Configuration

Kat reads configuration from a `.kat.toml` file located in the working directory (or a custom path via `--config`). All configuration is optional—any setting omitted falls back to Kat's built-in defaults.

```toml
# .kat.toml
[scan]
ignore = ["**/legacy/**", "**/*.tmp"]
extensions = ["png", "svg"]
use_default_ignores = true
```

## Merge order

1. Built-in defaults
   - Extensions: `png`, `jpg`, `jpeg`, `webp`, `gif`, `svg`
   - Ignore globs: `node_modules`, `android`, `ios`, `.expo`, `dist`, `build`, `target`, Python caches, and other common build artifacts.
2. Configuration file (`.kat.toml` or the file passed to `--config`).
3. CLI overrides (`--extensions`, `--ignore`, `--no-default-ignores`).

Values supplied later in the merge order take precedence. For example, running:

```bash
kat scan --ignore "**/renders/**" --no-default-ignores
```

drops the built-in ignore list entirely, but still respects any ignores defined inside `.kat.toml`.

## Settings reference

| Key                    | Type      | Description                                                                                |
| ---------------------- | --------- | ------------------------------------------------------------------------------------------ |
| `scan.ignore`          | array     | Extra glob patterns (Rust `globset` syntax) to exclude from scanning.                      |
| `scan.extensions`      | array     | File extensions (case-insensitive) that should be considered images.                      |
| `scan.use_default_ignores` | bool | Whether to merge Kat's default ignore list. Defaults to `true` when omitted.               |

If the `[scan]` table is omitted entirely, Kat behaves as if the config file were empty.

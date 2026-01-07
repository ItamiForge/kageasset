# Configuration

Create a `.kat.toml` file in your project root to customize scanning behavior.

```toml
[scan]
ignore = ["**/legacy/**", "**/*.tmp"]
extensions = ["png", "svg"]
use_default_ignores = true
```

## Settings

| Key | Type | Description |
|-----|------|-------------|
| `scan.ignore` | array | Glob patterns to exclude from scanning. |
| `scan.extensions` | array | File extensions to scan (case-insensitive). |
| `scan.use_default_ignores` | bool | Include built-in ignores (default: `true`). |

## Precedence

Settings are merged in this order (later overrides earlier):
1. Built-in defaults
2. `.kat.toml` file
3. CLI flags (`--ignore`, `--extensions`, `--no-default-ignores`)

Example: `kat scan --no-default-ignores` disables built-in ignores but still respects `.kat.toml` settings.

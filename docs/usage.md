# Kat Usage Guide

## Scan

```bash
# Scan current directory, write assets.md (Markdown) and assets.json (JSON)
kat scan --json

# Scan specific folders and write to a custom Markdown file
kat scan ./app ./packages/design --output reports/assets.md

# Limit to specific extensions and add ad-hoc ignores
kat scan -e png,svg --ignore "**/exports/**,*.tmp"

# Disable the built-in ignore list
kat scan --no-default-ignores

# Use a custom configuration file
kat scan --config ./configs/assets.toml
```

Notes:
- Markdown report is compact table output.
- JSON report contains the detailed metadata, hashes, warnings, and (if applicable) config path.
- Built-in ignore globs skip common build artifacts (`node_modules`, `android`, `ios`, `dist`, etc.).

## Info

```bash
kat info path/to/asset.png
```

## Duplicates

```bash
# Human-friendly summary
kat duplicates

# Machine-readable mode
kat duplicates --json
```

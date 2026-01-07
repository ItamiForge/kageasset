# Usage

## Scan

Scan directories and generate asset reports.

```bash
# Scan current directory (generates assets.md and assets.json)
kat scan --json

# Scan specific paths
kat scan ./app ./packages/design

# Custom output path
kat scan --output reports/assets.md

# Limit to specific extensions
kat scan -e png,svg

# Add ignore patterns
kat scan --ignore "**/exports/**,*.tmp"

# Disable default ignores
kat scan --no-default-ignores

# Use custom config file
kat scan --config ./.kat.toml
```

Output:
- **Markdown** (`assets.md`): Compact table with asset info.
- **JSON** (`assets.json`): Detailed metadata, hashes, and warnings.

## Info

Inspect a single asset file.

```bash
kat info path/to/image.png

# Include extended metadata
kat info path/to/image.png --meta
```

## Duplicates

Find duplicate assets by hash.

```bash
# Human-friendly output
kat duplicates

# JSON output
kat duplicates --json
```

# Kat Reports

Kat emits two report formats:

1. **Markdown (`assets.md`)** – a concise table meant for quick human review.
2. **JSON (`assets.json`, via `--json`)** – structured data for tooling and downstream processing.

## Markdown format

The Markdown report always starts with the total asset count followed by a table. Errors are displayed on the row beneath the asset.

```markdown
# Asset Report

Total assets: 3

| Path                     | Size   | Resolution | Format | Transparency |
|-------------------------|--------|------------|--------|---------------|
| images/splash.png       | 512.0 KB | 1920x1080 | PNG    | has alpha     |
| ↳ error                 | failed to hash images/splash.png: ... |            |        |               |
| graphics/logo.svg       | 24.0 KB  | -          | SVG    | has alpha     |
| textures/background.jpg | 1.2 MB   | 2048x1024  | JPEG   | opaque        |
```

Warnings (e.g., unreadable files) appear in a `## Warnings` section underneath the table.

## JSON format

When `--json` is supplied, Kat writes `assets.json` next to the Markdown report. The schema is intentionally small and versioned:

```json
{
  "schema_version": "0.1.0",
  "total_assets": 3,
  "assets": [
    {
      "relative_path": "images/splash.png",
      "absolute_path": "/full/path/images/splash.png",
      "size_bytes": 524288,
      "width": 1920,
      "height": 1080,
      "color_type": "RGBA",
      "has_alpha": true,
      "format": "PNG",
      "hash": "...",
      "error": null
    }
  ],
  "warnings": [],
  "config_path": null
}
```

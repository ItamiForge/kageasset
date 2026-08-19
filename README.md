# Kat

<p align="center">
  <img src="./icon.jpeg" alt="Kat logo" width="256" />
</p>

Fast asset inventory and image processing CLI. Catalog images, extract metadata, detect duplicates, and convert to SF Symbols.

> Catalog: [ItamiForge](https://itamiforge.github.io/itamiforge/docs/projects/#kat)

## Installation

```bash
cargo install --path crates/kat-cli
```

## Commands

```bash
kat scan                    # Scan directory for assets
kat info path/to/image.png  # Inspect single file
kat duplicates              # Find duplicate assets
kat sfsymbol icon.png       # Convert to SF Symbol
kat doctor                  # Check tool dependencies
```

## SF Symbol Pipeline

Converts raster images to Apple SF Symbol format:

1. **Normalize** — Center on square canvas (ImageMagick)
2. **Vectorize** — Extract alpha + trace (potrace)
3. **Optimize** — Clean paths (svgo)
4. **Package** — SF Symbol format (swiftdraw)

### Dependencies

| Tool | Install | Required |
|------|---------|:--------:|
| ImageMagick | `brew install imagemagick` | ✓ |
| potrace | `brew install potrace` | ✓ |
| svgo | `npm install -g svgo` | ✓ |
| swiftdraw | `brew install swiftdraw` | ✓ |
| bgone | `cargo install bgone` | `--remove-bg` only |

### Usage

```bash
kat sfsymbol icon.png                    # Transparent background
kat sfsymbol photo.jpg --remove-bg       # Remove background first
kat sfsymbol ./icons -o ./symbols        # Batch convert
kat sfsymbol icon.png --size 512         # Custom canvas size
```

## Project Structure

```
crates/
├── kat-core/      # Shared types and errors
├── kat-scanner/   # File discovery and metadata
├── kat-pipeline/  # Pipeline execution engine
├── kat-sfsymbol/  # SF Symbol stages
└── kat-cli/       # CLI interface
```

## License

MIT

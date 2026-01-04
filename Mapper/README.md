# Mapper

Crawls Yonsei University website to discover announcement boards for each department.

## Project Structure

```shell
Mapper/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── config.rs         # Configuration & constants
│   ├── error.rs          # Error types
│   ├── http.rs           # HTTP utilities
│   ├── models.rs         # Data structures
│   ├── selectors.rs      # CMS detection & selectors
│   └── crawlers/
│       ├── mod.rs
│       ├── departments.rs  # Campus/college/department crawler
│       └── boards.rs       # Board discovery logic
├── data/                 # Output directory
└── README.md
```

## Setup

Ensure you have Rust installed, then build:

```bash
cargo build --release
```

## Usage

```bash
cargo run --release
```

## Output Files

Generated in the `data/` directory:

| File | Description |
| ---- | ----------- |
| `yonsei_departments.json` | All departments with basic info |
| `yonsei_departments_boards.json` | Departments with discovered boards |
| `manual_review_needed.json` | Departments requiring manual review |

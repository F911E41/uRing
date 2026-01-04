# Mapper

Crawls Yonsei University website to discover announcement boards for each department.

## Project Structure

```shell
Mapper/
├── Cargo.toml
├── data/
│   ├── config.toml         # Runtime configuration
│   ├── seed.toml           # Campuses, keywords, CMS patterns
│   └── result/             # Output files
└── src/
    ├── main.rs             # Entry point
    ├── config.rs           # Configuration loading
    ├── error.rs            # Error types
    ├── models/
    │   ├── mod.rs
    │   ├── config.rs       # Config model structs
    │   ├── discovery.rs    # Discovery result structs
    │   └── seed.rs         # Seed data structs
    ├── services/
    │   ├── mod.rs
    │   ├── boards.rs       # Board discovery service
    │   ├── departments.rs  # Department crawler service
    │   └── selectors.rs    # CMS selector detection
    └── utils/
        ├── mod.rs
        ├── fs.rs           # File system utilities
        ├── http.rs         # HTTP client utilities
        └── url.rs          # URL manipulation
```

## Configuration

### `data/config.toml`

Runtime settings for HTTP, paths, discovery rules, and logging.

### `data/seed.toml`

Seed data including:

- **Campuses**: URLs and names of university campuses to crawl
- **Keywords**: Board name patterns to identify (e.g., "공지사항", "장학")
- **CMS Patterns**: CSS selectors for different CMS types

## Setup

```bash
cargo build --release
```

## Usage

```bash
cargo run --release
```

## Output Files

| File | Description |
| ---- | ----------- |
| `yonsei_departments.json` | All departments with basic info |
| `yonsei_departments_boards.json` | Departments with discovered boards |
| `manual_review_needed.json` | Departments requiring manual review |

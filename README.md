# uRing

Modern `Notice` application for Yonsei University

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
  - [Data Ingestion Pipeline](#data-ingestion-pipeline)
  - [Storage Strategy & Schema](#storage-strategy--schema)
  - [Notification & Retention Workflow](#notification--retention-workflow)
- [Roadmap](#roadmap)
  - [Todo](#todo)
- [License](#license)

## Overview

Being written in Rust, `uRing` Crawler is designed to efficiently crawl and parse notices from various university websites, focusing on Yonsei University.

## Architecture

### Data Ingestion Pipeline

The crawler operates on a **1-minute interval cron job**, polling various department portals to detect and extract the latest announcements. The retrieved payload is subsequently persisted to **AWS S3**.

### Storage Strategy & Schema

We utilize a **centralized S3 bucket** (`uRing/`) with a strict partitioning strategy to ensure efficient data management.

- **Root Prefix:** `uRing/`
- **Partitioning:** Monthly based (e.g., `s3://uRing/2023-01/`, `s3://uRing/2023-02/`)
- **Data Model:**
- **Monolithic JSON:** Instead of fragmenting data by department, all notices are aggregated into a single JSON file to simplify the read logic.

### Notification & Retention Workflow

To support real-time notification features, we implement a **Delta-First** approach:

1. **The `New` Directory (Hot Data):**

- Newly discovered notices are isolated and stored in a specific `New/` directory.
- This directory serves as the source of truth for triggering user notifications.

1. **Atomic Rotation:**

- Upon each crawl cycle, the existing content in `New/` is migrated to the appropriate **Monthly Archive** folder.
- The `New/` directory is then overwritten with the fresh batch of data.

## Roadmap

- **Detail Page Scaping:** The pipeline will be scaled to scrape, extract, and cache the full HTML content of individual notice detail pages into S3.

### Todo

- [ ] Enhance `Viewer` application to utilize the new data schema and support real-time notifications.
- [ ] Update documentation to reflect recent code changes.

## License

This project is licensed under the `MIT License` - see the [LICENSE](LICENSE) file for details.

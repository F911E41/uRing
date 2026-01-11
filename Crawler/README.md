# Crawler

Modern `Crawler` application for Yonsei University notices.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
  - [Data Ingestion Pipeline](#data-ingestion-pipeline)
  - [Storage Strategy & Schema](#storage-strategy--schema)
  - [Notification & Retention Workflow](#notification--retention-workflow)
- [Roadmap](#roadmap)
  - [Todo](#todo)

## Overview

Being written in Rust, `uRing Crawler` is designed to efficiently crawl and parse notices from various university websites, focusing on Yonsei University.

## Architecture

### Data Ingestion Pipeline

The crawler operates on a **1-minute interval cron job**, polling various department portals to detect and extract the latest announcements. The retrieved payload is subsequently persisted to **AWS S3**.

### Storage Strategy & Schema

We utilize a **centralized S3 bucket** (`uRing/`) with a strict partitioning strategy to ensure efficient data management.

- **Root Prefix:** `uRing/`
- **Partitioning:** Campus + monthly based (e.g., `s3://<bucket>/uRing/CampusA/2023-01/`)
- **Data Model:**
- **Monolithic JSON (per campus):** Instead of fragmenting data by department, all notices are aggregated into a single JSON file per campus to simplify the read logic.

### Notification & Retention Workflow

To support real-time notification features, we implement a **Delta-First** approach:

1. **The `New` Directory (Hot Data):**

- Newly discovered notices are isolated and stored in a specific `uRing/{campus}/New/` directory.
- This directory serves as the source of truth for triggering user notifications.

1. **Atomic Rotation:**

- Upon each crawl cycle, the existing content in `uRing/{campus}/New/` is migrated to the appropriate **Monthly Archive** folder.
- The `New/` directory is then overwritten with the fresh batch of data.

## Deployment (AWS Lambda)

1. Build the Lambda package (requires `cargo lambda`):
   - `cargo lambda build --release --arm64 --bin lambda --features lambda`
2. Upload the sitemap to S3:
   - `aws s3 cp data/output/yonsei_departments_boards.json s3://<bucket>/uRing/config/sitemap.json`
3. Deploy infrastructure:
   - `cd infra && terraform init`
   - `terraform apply -var="bucket_name=<bucket>" -var="region=ap-northeast-2"`

## Roadmap

- **Detail Page Scaping:** The pipeline will be scaled to scrape, extract, and cache the full HTML content of individual notice detail pages into S3.

### Todo

- [ ] Deploy the crawler to a production environment with proper monitoring.
- [ ] Enhance the CMS selector detection algorithm for better accuracy.

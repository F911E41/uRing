# Crawler

This document describes the internal architecture of the **uRing Crawler**, a serverless Rust-based crawler designed to ingest, deduplicate, and persist university notice data in a scalable and production-ready manner.

## Table of Contents

* [Design Principles](#design-principles)
* [High-Level Architecture](#high-level-architecture)
* [Data Ingestion Pipeline](#data-ingestion-pipeline)
* [Storage Strategy](#storage-strategy)
      - [S3 Bucket Layout (Snapshot-based)](#s3-bucket-layout-snapshot-based)
      - [Versioned Snapshot Generation](#versioned-snapshot-generation)
      - [Data Components](#data-components)
      - [Pointer-Based Atomic Rotation](#pointer-based-atomic-rotation)
      - [Cache Optimization](#cache-optimization)
* [Canonical Notice Identification](#canonical-notice-identification)
* [Concurrency & Idempotency](#concurrency--idempotency)
* [Retention & Lifecycle Management](#retention--lifecycle-management)
* [Deployment Model](#deployment-model)
* [Future Extensions](#future-extensions)
* [Summary](#summary)

## Design Principles

The crawler architecture is guided by the following principles:

* **Stateless Execution**: Optimized for AWS Lambda with no local state dependency.
* **Snapshot-based Persistence**: Each crawl result is stored as a versioned, immutable snapshot.
* **Delta-first Data Flow**: Designed to easily identify and broadcast new notices.
* **Structured Retrieval**: Clear separation between index, metadata, and detail data.
* **Operational Simplicity**: Using S3 as a structured database for high availability and low cost.

## High-Level Architecture

```text
[ EventBridge (10 min) ]
          |
          v
[ Lambda Crawler ]
          |
          v
[ S3 (Structured Snapshots + Version Pointer) ]
          |
          v
[ Consumer (API / App / Notification Service) ]

```

## Data Ingestion Pipeline

* The crawler is triggered on a 10-minute interval using **Amazon EventBridge**.
* Each execution fetches notices from configured department boards based on a predefined **siteMap**.
* Crawling is idempotent at the notice level using a canonical notice identifier.
* Results are bundled into a new `{version}` directory, representing a consistent state of the system at that time.

## Storage Strategy

### S3 Bucket Layout (Snapshot-based)

All data is organized under versioned snapshots to ensure atomicity and consistency for consumers.

```shell
config/
 ├─ config.toml                 # Crawler runtime configuration
 ├─ seed.toml                   # Crawler seed URLs and parameters
 └─ siteMap.json                # Crawler configuration used for this version

snapshots/{version}/
 ├── index/
 │    ├── all.json              # Global index of all active notices
 │    ├── campus/
 │    │    ├── seoul.json       # Campus-specific indices
 │    │    └── mirae.json
 │    └── category/
 │         └── academic.json    # Category-specific indices
 ├── meta/
 │    ├── campus.json           # Valid campus metadata
 │    ├── category.json         # Valid category metadata
 │    └── source.json           # Data source/origin mapping
 ├── detail/
 │    └── {noticeId}.json       # Full content of individual notices
 └── aux/
      ├── diff.json             # Changes compared to the previous version (the "Delta")
      └── stats.json            # Crawl statistics (count, latency, success rate)
```

### Versioned Snapshot Generation

During each crawl cycle, the crawler generates a new `{version}` (typically a timestamp or UUID).

* **Consistency**: All files within a version are guaranteed to be mutually consistent.
* **Immutability**: Once a version is written, it is never modified.
* **Granularity**: Consumers can choose to fetch only the `index/` for lists or `detail/` for specific content.

### Data Components

* **Index**: Read-optimized JSON lists containing minimal fields for rendering UI lists.
* **Meta**: Reference data that helps the frontend or API interpret campus and category IDs.
* **Detail**: The "Source of Truth" for each notice, containing the full body, attachments, and original metadata.
* **Aux (Delta-First)**: The `diff.json` provides a summary of added or updated notices, allowing notification services to trigger without comparing full indices.

### Pointer-Based Atomic Rotation

To ensure consumers always see a consistent state without relying on S3 directory listing performance:

* A **Pointer File** is maintained at the root: `latest.json`.
* It contains the key of the most recent successful `{version}`.
* **Atomic Switch**: Updating this single file "activates" the new snapshot globally.
* **Fallback**: In case of a crawl failure, the pointer remains at the previous version, ensuring the API/App never serves a broken state.

### Cache Optimization

* latest.json : max-age=2~10, stale-while-revalidate=...
* snapshots/{version}/index/* : max-age=1y, immutable
* snapshots/{version}/meta/* : max-age=1y, immutable
* snapshots/{version}/detail/* : max-age=1y, immutable
* snapshots/{version}/aux/* : max-age=1h, stale-while-revalidate=...

## Canonical Notice Identification

Each notice is assigned a deterministic identifier derived from stable attributes:

* **Campus & Department ID**
* **Original Notice Number/ID** from the source system
* **Stable URL**

This ensures reliable deduplication across multiple crawl cycles and maintains consistent `{noticeId}.json` filenames.

## Concurrency & Idempotency

* **Isolated Writes**: Since each execution writes to a unique `{version}` path, there is no risk of write-collision.
* **Idempotency**: Retrying a crawl for the same version (if manually triggered) will yield the same directory structure.

## Retention & Lifecycle Management

* **S3 Lifecycle Rules**:
* `snapshots/{version}/detail/`: Long-term retention (Event Storage).
* `snapshots/{version}/index/`: Short to medium retention (Read Cache).

* Old versions can be archived to Glacier or deleted after a set period, while the `latest` pointer ensures system continuity.

## Deployment Model

* **Runtime**: AWS Lambda (ARM64) binary built with `cargo lambda`.
* **Infrastructure**: Managed via Terraform.
* **Storage**: AWS S3 (Standard for active versions, Intelligent-Tiering for older ones).

## Future Extensions

* **Micro-Caching**: Serving the `index/` files via CloudFront with aggressive edge caching.
* **Incremental Crawling**: Using the previous version's indices to skip unchanged boards.
* **Full-Text Search**: Ingesting the `detail/` files into a lightweight search index (e.g., Meilisearch or OpenSearch).

## Summary

This architecture treats university notices as a series of immutable, versioned snapshots. By separating indices, details, and deltas into a structured S3 hierarchy, **uRing Crawler** achieves high reliability, easy auditability, and simple consumption for downstream services.

# Crawler

This document describes the internal architecture of the uRing Crawler, a serverless Rust-based crawler designed to ingest, deduplicate, and persist university notice data in a scalable and production-safe manner.

## Table of Contents

- [Design Principles](#design-principles)
- [High-Level Architecture](#high-level-architecture)
- [Data Ingestion Pipeline](#data-ingestion-pipeline)
- [Storage Strategy](#storage-strategy)
  - [S3 Bucket Layout](#s3-bucket-layout)
  - [Append-Only Event Storage](#append-only-event-storage)
  - [Snapshot Generation (Read-Optimized Views)](#snapshot-generation-read-optimized-views)
  - [Pointer-Based Atomic Rotation (Delta-First)](#pointer-based-atomic-rotation-delta-first)
- [Canonical Notice Identification](#canonical-notice-identification)
- [Concurrency & Idempotency](#concurrency--idempotency)
- [Retention & Lifecycle Management](#retention--lifecycle-management)
- [Deployment Model](#deployment-model)
- [Future Extensions](#future-extensions)
- [Summary](#summary)

## Design Principles

The crawler architecture is guided by the following principles:

- Stateless execution suitable for serverless environments (AWS Lambda)
- Append-only storage to ensure data integrity and auditability
- Delta-first data flow to support real-time notification use cases
- Clear separation between ingestion, storage, and consumption
- Operational simplicity over premature optimization

## High-Level Architecture

```code
[ EventBridge (10 min) ]
          |
          v
[ Lambda Crawler ]
          |
          v
[ S3 (Append-only Storage + Snapshot Pointer) ]
          |
          v
[ Consumer (API / App / Notification Service) ]
```

## Data Ingestion Pipeline

- The crawler is triggered on a 1-minute interval using Amazon EventBridge.
- Each execution fetches notices from configured department boards based on a predefined sitemap.
- Crawling is idempotent at the notice level using a canonical notice identifier.
- The crawler does not maintain any in-memory or persistent state between executions.

## Storage Strategy

### S3 Bucket Layout

All data is stored under a single logical namespace: uRing/.

```code
uRing/
 ├─ config/
 │   └─ sitemap.json
 │
 ├─ {campus}/
 │   ├─ events/
 │   │   └─ 2026-01/
 │   │       ├─ {notice_id}.json
 │   │       └─ ...
 │   │
 │   ├─ snapshots/
 │   │   ├─ 2026-01-12T11:03:00Z.json
 │   │   └─ ...
 │   │
 │   └─ new.pointer.json
```

### Append-Only Event Storage

- Each notice is persisted as an immutable event object:
  - uRing/{campus}/events/{yyyy-mm}/{notice_id}.json
- Once written, notice objects are never mutated or overwritten.
- This enables:
  - Safe retries
  - Historical inspection
  - Deterministic reprocessing

### Snapshot Generation (Read-Optimized Views)

- During each crawl cycle, the crawler aggregates newly discovered notices into a snapshot file:
- uRing/{campus}/snapshots/{timestamp}.json
- Snapshots are optimized for read simplicity, not write performance.
- Snapshots represent the current “hot view” of recent notices for a campus.

### Pointer-Based Atomic Rotation (Delta-First)

To support notification workflows without relying on non-atomic S3 directory operations:

- A small pointer file is maintained:
- uRing/{campus}/new.pointer.json
- The pointer contains only the S3 key of the latest snapshot.
- Consumers:

 1. Read new.pointer.json
 2. Fetch the referenced snapshot

- Updating the pointer is the only overwrite operation, making the rotation effectively atomic.

This replaces directory move/overwrite patterns and avoids intermediate inconsistent states.

## Canonical Notice Identification

Each notice is assigned a canonical, deterministic identifier derived from stable attributes such as:

- campus
- department_id
- board_id
- original notice URL

This ensures:

- Reliable deduplication
- Idempotent writes
- Stability across retries and minor content changes

## Concurrency & Idempotency

- The crawler is designed to tolerate overlapping or delayed executions.
- Idempotency is enforced at the notice storage level.
- Optional campus-level locking (e.g., DynamoDB conditional writes or S3-based markers) can be enabled to prevent redundant snapshot generation.

## Retention & Lifecycle Management

- S3 lifecycle rules are applied per prefix:
- events/: long-term retention
- snapshots/: short to medium retention
- Old snapshots can be safely expired without affecting historical data.
- Pointer files always reference a valid snapshot.

## Deployment Model

- The crawler runs as an AWS Lambda (ARM64) binary built using cargo lambda.
- Infrastructure is managed via Terraform.
- No persistent compute or database is required.

## Future Extensions

- Detail page scraping with HTML content cached alongside notice events
- Change detection at the content level (diff-based updates)
- Downstream fan-out via SQS/SNS for large-scale notification delivery
- Metrics and tracing via CloudWatch (latency, delta size, error rate)

## Summary

This architecture treats notice data as immutable events, separates write-optimized and read-optimized paths, and uses pointer-based rotation to achieve atomicity on top of S3.

The result is a crawler that is:

- Robust under retries
- Safe under concurrency
- Simple to consume
- Easy to evolve

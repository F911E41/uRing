# uRing

> A High-Performance, Serverless Notice Aggregator for Yonsei University.

`uRing` is a modern, distributed crawling engine designed to ingest, deduplicate, and broadcast university notices at scale. Built with **Rust** and **AWS Serverless** primitives, it prioritizes cost-efficiency, operational resilience, and massive concurrency.

## Table of Contents

* [Overview](#overview)
* [Architecture](#architecture)
  * [The Fan-Out Pipeline](#the-fan-out-pipeline)
  * [Storage Strategy (CAS)](#storage-strategy-cas)
  * [Notification & Retention Workflow](#notification--retention-workflow)

* [Tech Stack](#tech-stack)
* [Roadmap](#roadmap)
* [License](#license)

## Overview

University notice boards are fragmented, often slow, and inconsistent. `uRing` solves this by treating notice ingestion as a distributed data pipeline rather than a simple cron job.

Unlike traditional scrapers, `uRing` utilizes a **Content-Addressable Storage (CAS)** model to minimize storage costs and separates the *structure* of data from the *content*. This allows the system to crawl hundreds of department boards simultaneously using a Fan-Out architecture without storage bottlenecks or race conditions.

## Architecture

The system operates on a "Snapshot" basis, where every crawl cycle produces a versioned, immutable view of the university's notice ecosystem.

### The Fan-Out Pipeline

The ingestion process is decoupled into three distinct stages to ensure scalability:

**Orchestrator (Scheduling)**:

* Triggered via **Amazon EventBridge**.
* Reads the global `siteMap.json` and dispatches distinct crawl jobs to an **SQS FIFO Queue**.
* *Role:* Logistics & Scheduling.

**Workers (Execution)**:

* Scalable **Rust Lambda** functions consume the SQS queue.
* Each worker fetches, parses, and hashes notice content.
* **Deduplication**: Checks the global Blob Store before writing. If the content hash exists, it only writes a lightweight reference.
* *Role:* Heavy Lifting & I/O.

**Aggregator (Commit)**:

* Triggered upon batch completion.
* Consolidates partial results into a unified **Snapshot Manifest**.
* Calculates the "Diff" (Delta) for notification services.
* Updates the atomic `latest.json` pointer to make the new snapshot live.
* *Role:* Consistency & Finalization.

### Storage Strategy (CAS)

`uRing` uses S3 not just as a file system, but as a structured database using **Content-Addressable Storage**.

#### 1. The Blob Store (`/blobs`)

The single source of truth for content. Files are named by their SHA-256 hash.

* **Efficiency**: A notice posted on the "Computer Science" board and cross-posted to "Engineering" is stored physically only once.
* **Immutability**: Blobs are never overwritten, only created.

#### 2. The Snapshot Layer (`/snapshots/{version}`)

Lightweight JSON manifests that map specific Board IDs to Blob Hashes.

* **Indices**: Optimized for frontend rendering (pagination, filtering).
* **Diffs**: `aux/diff.json` allows downstream consumers to see exactly what changed since the last version without scanning the whole DB.

### Notification & Retention Workflow

* **Hot Storage (Standard S3)**: The `latest` snapshot and the last 24 hours of versions.
* **Warm Storage (Infrequent Access)**: Historical snapshots for audit logs.
* **Notification**: Downstream services (Push Notifications, Email) subscribe to the `diff.json` generation event, ensuring users are only alerted for truly *new* content.

## Tech Stack

* **Core Logic**: Rust (optimizing for cold-start performance and memory safety)
* **Compute**: AWS Lambda (ARM64/Graviton2)
* **Orchestration**: Amazon EventBridge & SQS (FIFO)
* **Storage**: Amazon S3 (Intelligent-Tiering)
* **Infrastructure**: Terraform (IaC)

## Roadmap

### Phase 1: Core Engine (Current)

* [ ] Implement basic crawler logic in Rust
* [ ] Migrate to Fan-Out architecture (Orchestrator/Worker split)
* [ ] Implement CAS (Content-Addressable Storage) logic
* [ ] Set up Terraform for SQS & Lambda wiring

### Phase 2: Resilience & Optimization

* [ ] Add Dead Letter Queues (DLQ) for failed crawls
* [ ] Implement `errors.json` manifest for partial availability
* [ ] Configure CloudFront with OAC (Origin Access Control)

### Phase 3: Search & Discovery

* [ ] Event-driven Indexing pipeline (S3 Event -> Search Indexer Lambda)
* [ ] Integration with Meilisearch or AWS OpenSearch

## License

> This project is licensed under the `MIT License`

See the [LICENSE](LICENSE) file for details.

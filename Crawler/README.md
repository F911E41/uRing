# uRing Crawler

This document describes the internal architecture of the **uRing Crawler**, a serverless, event-driven system designed to ingest university notice data. This version introduces **Content-Addressable Storage (CAS)** and a **Fan-Out/Fan-In** execution model to maximize deduplication, concurrency, and fault tolerance.

## Table of Contents

* [Design Principles](#design-principles)
* [High-Level Architecture](#high-level-architecture)
* [The Fan-Out Ingestion Pipeline](#the-fan-out-ingestion-pipeline)
* [Storage Strategy: CAS & Deduplication](#storage-strategy-cas--deduplication)
  * [The Blob Store (Content)](#the-blob-store-content)
  * [The Snapshot Layer (Structure)](#the-snapshot-layer-structure)
  * [Data Components](#data-components)
* [Resilience & Error Handling](#resilience--error-handling)
* [Security & Access Control](#security--access-control)
* [Search Integration](#search-integration)
* [Summary](#summary)

## Design Principles

* **Content-Addressable Persistence**: Data is stored based on its content hash (SHA-256), not its location. This eliminates 99% of redundant writes.
* **Asynchronous Fan-Out**: Crawl jobs are distributed across concurrent workers via queues, decoupled from the scheduling logic.
* **Partial Availability**: The system is designed to succeed even if specific target sites fail. Errors are cataloged, not fatal.
* **Edge-First Delivery**: All reads are served via a CDN (CloudFront) with aggressive caching policies.

## High-Level Architecture

The system moves from a monolithic Lambda to a distributed **Orchestrator-Worker-Aggregator** pattern.

```text
[ EventBridge (Cron) ]
          |
          v
[ Orchestrator Lambda ] --(Batch Jobs)--> [ SQS FIFO Queue ]
                                                 |
                                                 v
                                        [ Worker Lambdas (Scale n...) ]
                                                 | (Write Blobs & Fragments)
                                                 v
                                        [ S3 (Staging & Blob Store) ]
                                                 |
                                         (Event / Finalize)
                                                 v
                                        [ Aggregator Lambda ]
                                                 |
                                                 v
                                        [ S3 (Final Indices & Pointer) ]

```

## The Fan-Out Ingestion Pipeline

The crawl cycle is broken down into three distinct phases to ensure scalability:

### 1. The Orchestrator (Scheduling)

* **Trigger**: Runs every 10 minutes via EventBridge.
* **Responsibility**: Reads the `siteMap.json`. It does **not** fetch external websites.
* **Action**: Splits the sitemap into discrete "Job Messages" (e.g., `Crawl { Dept: CS, URL: ... }`) and pushes them to an Amazon SQS FIFO queue to ensure exactly-once delivery.

### 2. The Workers (Execution)

* **Trigger**: SQS Lambda Event Source Mapping (scales automatically based on queue depth).

* **Responsibility**:

  1. Fetch HTML from the target URL.
  2. Parse notices.
  3. Compute **SHA-256 Hash** of the notice body.
  4. **CAS Check**: If `blobs/sha256/{hash}.json` exists, skip writing the body. If not, write it.
  5. Write a lightweight **Notice Fragment** (metadata + hash reference) to a temporary staging area in S3.

### 3. The Aggregator (Commit)

* **Trigger**: Triggered once the SQS queue is drained or by a "EndOfBatch" message.

* **Responsibility**:

  1. Reads all Notice Fragments from the staging area.
  2. Generates the aggregate `index/` files and `aux/diff.json`.
  3. Writes the versioned snapshot.
  4. Updates the `latest.json` pointer to make the new data live.

## Storage Strategy: CAS & Deduplication

We separate the **Content** (Heavy, rarely changes) from the **Structure** (Light, changes often).

### The Blob Store (Content)

This directory acts as a global database of unique notices. Files here are never deleted (unless via long-term lifecycle policies) and never overwritten.

```shell
blobs/
 └─ sha256/
     ├─ a1b2...e4.json   # Actual content of a notice
     ├─ c9d8...f1.json   # Content of another notice
     └─ ...

```

### The Snapshot Layer (Structure)

Snapshots now act as **Manifests**. They map canonical IDs to Content Hashes. This makes the snapshots incredibly small and fast to generate.

```shell
snapshots/{version}/
 ├── index/
 │    ├── all.json       # List: [{ "id": 101, "title": "...", "content_hash": "a1b2..." }]
 │    └── campus/
 │         └── seoul.json
 ├── meta/               # (Unchanged from previous design)
 ├── aux/
 │    ├── diff.json      # Delta: "Notice 101 changed hash A -> hash B"
 │    └── errors.json    # NEW: Log of specific crawl failures

```

### Data Components

| Component | Function | Storage Location | Cache Strategy |
| --- | --- | --- | --- |
| **Pointer** | Points to active version | `root/latest.json` | `max-age=10` |
| **Index** | Light UI metadata + Hash Refs | `snapshots/{v}/index/` | `immutable` |
| **Blob** | The full notice body | `blobs/sha256/{hash}` | `immutable` |
| **Delta** | Notification triggers | `snapshots/{v}/aux/diff` | `max-age=60` |

## Resilience & Error Handling

In a distributed crawl, we must assume some targets will be slow or down.

* **Dead Letter Queue (DLQ)**: If a Worker Lambda crashes or times out repeatedly on a specific URL, the message is moved to an SQS DLQ. This prevents "poison pill" notices from blocking the queue.

* **Error Manifest (`errors.json`)**:
  * Instead of failing the entire snapshot, partial failures are acceptable.
  * Failed boards are logged in `snapshots/{version}/aux/errors.json`.
  * The frontend can use this to display a "Data may be incomplete for [Department Name]" warning.

* **Stale-Data Fallback**: If a specific board fails to crawl, the Aggregator can optionally copy the *previous version's* fragments for that board into the current snapshot, ensuring the UI remains populated.

## Security & Access Control

* **No Public S3**: The S3 bucket strictly blocks public access.

* **CloudFront Distribution**:
  * Acts as the only gateway to the data.
  * Uses **Origin Access Control (OAC)** to authenticate with S3.
  * Enforces HTTPS/TLS 1.3.

* **WAF (Web Application Firewall)**:
  * Attached to CloudFront.
  * Rate limits IP addresses to prevent scraping abuse.

* **CORS**: Configured at the CloudFront level to allow only specific app origins.

## Search Integration

Full-text search is decoupled from the crawl loop to ensure performance.

  1. **Event**: S3 emits an event `s3:ObjectCreated:Put` when a new file lands in `blobs/`.
  2. **Indexer Lambda**: A small Lambda triggers on this event.
  3. **Action**: It pushes the JSON document into **Meilisearch** or **AWS OpenSearch**.
  4. **Result**: The search index is updated near real-time, only processing *new* or *changed* content (deduplicated by the CAS nature of the `blobs` directory).

## Summary

By adopting **Content-Addressable Storage**, the uRing Crawler reduces S3 write costs by orders of magnitude. By moving to a **Fan-Out architecture**, it gains the ability to crawl hundreds of departments in parallel without timeout risks. This architecture represents a mature, resilient foundation ready for production scale.

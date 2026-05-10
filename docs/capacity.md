# Capacity & Limitations

This document describes the current capacity limits of `jot` in its default single-instance configuration.

## Design target

`jot` is designed for **personal use**: a single identity, 1–5 devices, a few hundred notes.
It is **not** designed for multi-tenant, high-availability, or high-throughput deployments
without the infrastructure changes described at the bottom.

## Current limits

| Dimension | Estimated limit | Root cause |
|---|---|---|
| Concurrent HTTP requests | ~50–100 req/s sustained | SQLite WAL serialises all writes |
| Concurrent WebSocket connections | ~1 000 | Tokio async; RAM is the only constraint (~2 KB/conn) |
| Note size | No application limit | Blobs are files on disk; limited by available disk space |
| Boards per identity | No application limit | SQLite handles millions of rows |
| Notes per board | No application limit | Same |
| Blob write throughput | ~200 writes/s | Sequential I/O on `LocalStore` (single directory) |
| Number of identities | No limit | All isolated by `identity_id` FK |
| Multi-instance / HA | **Not supported** | No replication; SQLite and `LocalStore` are local |

## Bottleneck details

### SQLite (writes)
Axum uses a `SqlitePool` (sqlx) backed by a single file. In WAL mode (default), concurrent
reads are fine but writes are serialised. Under write-heavy workloads (bulk note creation)
throughput degrades quickly.

**Mitigation for higher load**: replace `storage::Db` with a PostgreSQL-backed implementation.
The `Db` trait surface is small — all queries are in `crates/storage/src/db/`.

### `LocalStore` (blob storage)
Blobs are stored as flat files under `~/.local/share/jot/blobs/`. No CDN, no deduplication,
no chunking. Large files (images, audio) will saturate local I/O.

**Mitigation**: implement the `BlobStore` trait backed by S3-compatible object storage.
The trait is defined in `crates/storage/src/blobs/mod.rs`.

### WebSocket broadcast
`ws_tx` is a `tokio::sync::broadcast::Sender<WsEvent>` with a fixed channel capacity.
If a slow consumer can't keep up, it receives a `Lagged` error and is disconnected.
The current capacity is 16 messages (broadcast default). Under burst events (many notes
created at once) some subscribers may miss events and need to poll.

### No horizontal scaling
The JWT signing key, SQLite file, and blob directory are all local to the server process.
Running multiple instances would require:
- Shared signing key (already externalised to a PEM file — can be volume-mounted)
- Shared database (PostgreSQL)
- Shared blob storage (S3)
- A load balancer that forwards `/ws` connections with sticky sessions (or switch to Redis pub/sub for events)

## Tested configuration

All 84 workspace tests pass under the default single-instance SQLite configuration.
No load testing has been performed.

## Roadmap for scale

| Priority | Change | Impact |
|---|---|---|
| 1 | PostgreSQL backend | 10–100× write throughput |
| 2 | S3 blob storage | Unlimited blob capacity, CDN-ready |
| 3 | Redis pub/sub for WS events | Multi-instance WebSocket fan-out |
| 4 | Rate limiting (tower middleware) | Protect single-instance from abuse |

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Bilibili Hot Video Analysis Platform v2 — a data pipeline and dashboard for analyzing trending Bilibili videos. Three independent sub-projects (no monorepo/workspace):

- **crawler/** — Rust binary that fetches ranking data, video details, comments, danmaku, and UP info from Bilibili APIs
- **backend/** — Rust Axum HTTP API serving analytics data to the frontend
- **frontend/** — React + TypeScript + Vite SPA dashboard (ECharts + Tailwind CSS)
- **etl/** — Python scripts that transform crawler NDJSON output into MySQL (local PyMySQL or Spark-based)

## Build & Dev Commands

No root-level build system. Each sub-project is independent:

```bash
# Frontend
cd frontend && npm run dev        # Vite dev server on :5173, proxies /api → localhost:9090
cd frontend && npm run build      # tsc -b && vite build → frontend/dist/
cd frontend && npm run lint       # ESLint

# Backend
cd backend && cargo build --release   # → target/release/bilibili-backend
cd backend && cargo test

# Crawler
cd crawler && cargo build --release   # → target/release/bilibili-crawler
cd crawler && cargo test              # Extensive tests in state.rs, config.rs, wbi.rs, etc.

# ETL
cd etl && python main.py [YYYYMMDD]   # Local ETL (defaults to yesterday)
```

## Architecture & Data Flow

```
Bilibili APIs → Crawler (Rust) → NDJSON files (~/output/YYYYMMDD_HHMM/)
                                        ↓
                              upload_to_hdfs.sh → HDFS
                                        ↓
                              Spark ETL or Local ETL → MySQL (bilibili database)
                                        ↓
                              Backend (Axum) → REST API on :9090
                                        ↓
                              Frontend (React) → Dashboard SPA
```

Deployment: nginx serves frontend SPA + proxies `/api` to backend on :9090. FRP reverse proxy exposes services through a public server.

## Key Environment Variables

| Variable | Component | Purpose |
|----------|-----------|---------|
| `BILI_COOKIE` | Crawler | Bilibili session cookie (required) |
| `DATABASE_URL` | Backend | MySQL connection string (default: `mysql://root:root@127.0.0.1:3307/bilibili`) |
| `API_TOKEN` | Backend | Optional Bearer token for API auth |
| `BIND` | Backend | Listen address (default: `0.0.0.0:9090`) |
| `DB_HOST/DB_USER/DB_PASSWORD/DB_NAME` | ETL | MySQL connection (default: localhost/root//bilibili) |

## Database Schema

9 MySQL tables in `bilibili` database. Core tables: `hot_video` (main video data, PK: `bvid, snapshot_date`), `video_interaction` (engagement rates), `comment_info`, `danmaku_info`, `up_info`, `word_cloud_*` (3 tables for title/comment/danmaku word clouds), `word_trend`, `snapshot_dates`. Schema DDL at `scripts/init_db.sql`.

Heat Score ranking formula:
```
LN(view_count)*3.0 + LN(like_count)*3.5 + LN(coin_count)*3.0 + LN(favorite_count)*4.0 + LN(reply_count)*1.0 + LN(share_count)*2.0
```

## Backend API (Axum)

13 GET endpoints under `/api/`. All handlers are in `backend/src/handlers.rs` with raw SQL queries (sqlx). Models in `models.rs`. Routes defined in `main.rs`.

Key endpoints: `/api/overview`, `/api/hot-videos` (paginated/filterable), `/api/partition-compare`, `/api/lifecycle`, `/api/word-cloud`, `/api/hot-comments`, `/api/interaction`, `/api/word-trend`, `/api/snapshot-dates`.

Uses sqlx offline query checking — cached query metadata in `backend/.sqlx/`. Run `cargo sqlx prepare` after changing SQL queries.

## Frontend (React + Vite)

Two screens in `App.tsx`: `overview` (KPI cards, partition bar, hot video table, lifecycle chart, word cloud, comments, interaction radar) and `analysis` (partition metric matrix, word trend, partition detail). Global state via `DashboardContext`. API client in `api/index.ts` (Axios with Bearer auth).

Vite proxies `/api` to `http://localhost:9090` in dev mode.

## Crawler Architecture

Main loop in `main.rs`: checks if full crawl needed today → runs full crawl then sleeps until tomorrow's trigger hour; otherwise runs incremental crawl every 600s until stop hour (21:00).

Key modules: `api/` (Bilibili API clients for ranking/video/comment/danmaku/up), `scheduler/` (full + incremental crawl with Semaphore concurrency), `storage/ndjson.rs` (NDJSON writer), `wbi.rs` (WBI v3 signature for API auth), `state.rs` (SQLite state machine for dedup/retry). Protobuf for danmaku parsing via `prost`.

## Cross-Compilation

Both Rust crates target `x86_64-unknown-linux-musl` for static linking to Linux servers. Config in `backend/.cargo/config.toml` and `crawler/.cargo/config.toml`.

## Hive Data Warehouse

Optional 4-layer architecture (ODS→DWD→DWS→ADS) on HDFS for Spark-based ETL. DDL at `scripts/hive_ddl_full.sql`. The Spark ETL (`etl/spark_etl.py`) reads from HDFS and writes to MySQL via JDBC.

## Git Conventions

Commits use conventional commit style (`feat:`, `docs:`). The `archive/` directory contains the v1 Python project — do not modify.

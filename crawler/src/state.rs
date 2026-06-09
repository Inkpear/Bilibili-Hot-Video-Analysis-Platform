use chrono::Utc;
use rusqlite::{params, Connection};

use crate::error::Result;

/// Lightweight row returned by `get_video_info`.
#[derive(Debug, Clone)]
pub struct VideoRow {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub has_detail: bool,
    pub has_comments: bool,
    pub has_danmaku: bool,
    pub fail_count: u32,
}

/// SQLite-backed state manager for crawler dedup / retry tracking.
pub struct StateManager {
    conn: Connection,
}

impl StateManager {
    // ── Construction ──────────────────────────────────────────────────

    /// Open (or create) the SQLite database at `db_path` and ensure all
    /// required tables exist.
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let sm = Self { conn };
        sm.create_tables()?;
        Ok(sm)
    }

    /// Convenience constructor that takes an already-opened connection
    /// (used by tests with `open_in_memory`).
    #[allow(dead_code)]
    fn from_conn(conn: Connection) -> Result<Self> {
        let sm = Self { conn };
        sm.create_tables()?;
        Ok(sm)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS collected_videos (
                bvid            TEXT PRIMARY KEY,
                aid             INTEGER DEFAULT 0,
                cid             INTEGER DEFAULT 0,
                title           TEXT    DEFAULT '',
                category        TEXT    DEFAULT '',
                sub_category    TEXT    DEFAULT '',
                duration        INTEGER DEFAULT 0,
                danmaku_count   INTEGER DEFAULT 0,
                up_id           TEXT    DEFAULT '',
                has_detail      INTEGER DEFAULT 0,
                has_comments    INTEGER DEFAULT 0,
                has_danmaku     INTEGER DEFAULT 0,
                fail_count      INTEGER DEFAULT 0,
                last_error      TEXT,
                last_attempt    INTEGER,
                first_seen      DATE,
                created_at      INTEGER,
                updated_at      INTEGER
            );

            CREATE TABLE IF NOT EXISTS collected_comments (
                comment_id  INTEGER PRIMARY KEY,
                bvid        TEXT NOT NULL,
                collect_time INTEGER
            );

            CREATE TABLE IF NOT EXISTS collected_danmaku (
                danmaku_id  INTEGER PRIMARY KEY,
                bvid        TEXT NOT NULL,
                collect_time INTEGER
            );

            CREATE TABLE IF NOT EXISTS collected_ups (
                up_id       TEXT PRIMARY KEY,
                collect_time INTEGER
            );

            CREATE TABLE IF NOT EXISTS failed_ups (
                up_id       TEXT PRIMARY KEY,
                retry_count INTEGER DEFAULT 0,
                last_error  TEXT,
                last_attempt INTEGER
            );

            CREATE TABLE IF NOT EXISTS snapshot_log (
                snapshot_id     TEXT PRIMARY KEY,
                video_count     INTEGER DEFAULT 0,
                comment_count   INTEGER DEFAULT 0,
                danmaku_count   INTEGER DEFAULT 0,
                up_count        INTEGER DEFAULT 0,
                status          TEXT    DEFAULT 'pending',
                started_at      INTEGER,
                completed_at    INTEGER
            );
            ",
        )?;
        Ok(())
    }

    // ── Video methods ─────────────────────────────────────────────────

    /// Insert a video row. Returns `true` if the row was newly inserted,
    /// `false` if `bvid` already existed.
    pub fn insert_video(
        &self,
        bvid: &str,
        title: &str,
        category: &str,
        sub_category: &str,
        aid: i64,
        cid: i64,
        duration: i32,
        danmaku_count: i64,
        up_id: &str,
        _snapshot_id: &str,
    ) -> Result<bool> {
        let now = Utc::now().timestamp();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO collected_videos
                (bvid, aid, cid, title, category, sub_category, duration,
                 danmaku_count, up_id, first_seen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                bvid,
                aid,
                cid,
                title,
                category,
                sub_category,
                duration,
                danmaku_count,
                up_id,
                today,
                now,
                now,
            ],
        )?;
        Ok(self.conn.changes() > 0)
    }

    /// Mark that the full detail for this video has been collected.
    pub fn mark_detail_done(&self, bvid: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE collected_videos SET has_detail = 1, updated_at = ?1 WHERE bvid = ?2",
            params![now, bvid],
        )?;
        Ok(())
    }

    /// Mark that comments for this video have been collected.
    pub fn mark_comments_done(&self, bvid: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE collected_videos SET has_comments = 1, updated_at = ?1 WHERE bvid = ?2",
            params![now, bvid],
        )?;
        Ok(())
    }

    /// Mark that danmaku for this video have been collected.
    pub fn mark_danmaku_done(&self, bvid: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE collected_videos SET has_danmaku = 1, updated_at = ?1 WHERE bvid = ?2",
            params![now, bvid],
        )?;
        Ok(())
    }

    /// Increment `fail_count` and record the error. Returns the new count.
    pub fn increment_fail(&self, bvid: &str, error: &str) -> Result<u32> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE collected_videos
             SET fail_count = fail_count + 1,
                 last_error = ?1,
                 last_attempt = ?2,
                 updated_at  = ?2
             WHERE bvid = ?3",
            params![error, now, bvid],
        )?;
        let count: u32 = self.conn.query_row(
            "SELECT fail_count FROM collected_videos WHERE bvid = ?1",
            params![bvid],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Reset failure tracking for a video.
    pub fn reset_fail(&self, bvid: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE collected_videos
             SET fail_count = 0, last_error = NULL, updated_at = ?1
             WHERE bvid = ?2",
            params![now, bvid],
        )?;
        Ok(())
    }

    /// Retrieve basic video row info, or `None` if `bvid` is absent.
    pub fn get_video_info(&self, bvid: &str) -> Result<Option<VideoRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT bvid, aid, cid, has_detail, has_comments, has_danmaku, fail_count
             FROM collected_videos WHERE bvid = ?1",
        )?;
        let mut rows = stmt.query_map(params![bvid], |row| {
            Ok(VideoRow {
                bvid: row.get(0)?,
                aid: row.get(1)?,
                cid: row.get(2)?,
                has_detail: row.get::<_, i32>(3)? != 0,
                has_comments: row.get::<_, i32>(4)? != 0,
                has_danmaku: row.get::<_, i32>(5)? != 0,
                fail_count: row.get::<_, u32>(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Return bvids that still need work and have not exceeded `max_fail`.
    pub fn get_failed_bvids(&self, max_fail: u32) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT bvid FROM collected_videos
             WHERE (has_detail = 0 OR has_comments = 0 OR has_danmaku = 0)
               AND fail_count < ?1",
        )?;
        let rows = stmt.query_map(params![max_fail], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    /// From the given `bvids`, return those that are **not** yet in the DB.
    pub fn get_new_bvids_for_check(&self, bvids: &[String]) -> Result<Vec<String>> {
        if bvids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (0..bvids.len()).map(|i| format!("?{}", i + 1)).collect();
        let sql = format!(
            "SELECT bvid FROM collected_videos WHERE bvid IN ({})",
            placeholders.join(", ")
        );
        let known: std::collections::HashSet<String> = {
            let mut stmt = self.conn.prepare(&sql)?;
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            for b in bvids {
                params_vec.push(Box::new(b.clone()));
            }
            let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(refs.as_slice(), |row| row.get::<_, String>(0))?;
            let mut set = std::collections::HashSet::new();
            for r in rows {
                set.insert(r?);
            }
            set
        };
        Ok(bvids
            .iter()
            .filter(|b| !known.contains(b.as_str()))
            .cloned()
            .collect())
    }

    /// Returns `true` when detail, comments, and danmaku are all collected.
    pub fn video_done(&self, bvid: &str) -> Result<bool> {
        let done: bool = self.conn.query_row(
            "SELECT (has_detail = 1 AND has_comments = 1 AND has_danmaku = 1)
             FROM collected_videos WHERE bvid = ?1",
            params![bvid],
            |row| row.get(0),
        )?;
        Ok(done)
    }

    // ── Comment methods ───────────────────────────────────────────────

    /// Check whether a comment has already been recorded.
    pub fn is_comment_collected(&self, comment_id: i64) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_comments WHERE comment_id = ?1",
            params![comment_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Batch-insert comment ids. Each tuple is `(comment_id, bvid)`.
    pub fn insert_comments_batch(&self, ids: &[(i64, &str)]) -> Result<()> {
        let now = Utc::now().timestamp();
        let mut stmt = self.conn.prepare(
            "INSERT OR IGNORE INTO collected_comments (comment_id, bvid, collect_time)
             VALUES (?1, ?2, ?3)",
        )?;
        for (cid, bvid) in ids {
            stmt.execute(params![cid, bvid, now])?;
        }
        Ok(())
    }

    /// Return all collected comment ids for a given video.
    pub fn get_collected_comment_ids(&self, bvid: &str) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT comment_id FROM collected_comments WHERE bvid = ?1",
        )?;
        let rows = stmt.query_map(params![bvid], |row| row.get::<_, i64>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ── Danmaku methods ───────────────────────────────────────────────

    /// Check whether a danmaku has already been recorded.
    pub fn is_danmaku_collected(&self, danmaku_id: i64) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_danmaku WHERE danmaku_id = ?1",
            params![danmaku_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Batch-insert danmaku ids. Each tuple is `(danmaku_id, bvid)`.
    pub fn insert_danmaku_batch(&self, ids: &[(i64, &str)]) -> Result<()> {
        let now = Utc::now().timestamp();
        let mut stmt = self.conn.prepare(
            "INSERT OR IGNORE INTO collected_danmaku (danmaku_id, bvid, collect_time)
             VALUES (?1, ?2, ?3)",
        )?;
        for (did, bvid) in ids {
            stmt.execute(params![did, bvid, now])?;
        }
        Ok(())
    }

    /// Return all collected danmaku ids for a given video.
    pub fn get_collected_danmaku_ids(&self, bvid: &str) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT danmaku_id FROM collected_danmaku WHERE bvid = ?1",
        )?;
        let rows = stmt.query_map(params![bvid], |row| row.get::<_, i64>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ── UP methods ────────────────────────────────────────────────────

    /// Check whether an UP has already been collected.
    pub fn is_up_collected(&self, up_id: &str) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_ups WHERE up_id = ?1",
            params![up_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Record an UP as collected (idempotent).
    pub fn mark_up_collected(&self, up_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR IGNORE INTO collected_ups (up_id, collect_time) VALUES (?1, ?2)",
            params![up_id, now],
        )?;
        Ok(())
    }

    /// Returns `true` when the UP should be skipped:
    /// `retry_count >= 3` **and** the last attempt was within the past 24 h.
    pub fn should_skip_up(&self, up_id: &str) -> Result<bool> {
        let now = Utc::now().timestamp();
        let day_ago = now - 86_400;
        let skip: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM failed_ups
             WHERE up_id = ?1 AND retry_count >= 3 AND last_attempt > ?2",
            params![up_id, day_ago],
            |row| row.get(0),
        )?;
        Ok(skip)
    }

    /// Record (or bump) a failure for an UP.
    pub fn record_up_failure(&self, up_id: &str, error: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO failed_ups (up_id, retry_count, last_error, last_attempt)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(up_id) DO UPDATE SET
                 retry_count = retry_count + 1,
                 last_error  = excluded.last_error,
                 last_attempt = excluded.last_attempt",
            params![up_id, error, now],
        )?;
        Ok(())
    }

/// Reset daily flags so every full crawl re-collects comments/danmaku.
    pub fn reset_daily_flags(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE collected_videos SET has_comments = 0, has_danmaku = 0",
            [],
        )?;
        Ok(())
    }
    // ── Snapshot methods ──────────────────────────────────────────────

    /// Create a new snapshot entry with status `'pending'`.
    pub fn create_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR IGNORE INTO snapshot_log (snapshot_id, started_at) VALUES (?1, ?2)",
            params![snapshot_id, now],
        )?;
        Ok(())
    }

    /// Finalise a snapshot with counts and a terminal status.
    pub fn complete_snapshot(
        &self,
        snapshot_id: &str,
        video_count: i64,
        comment_count: i64,
        danmaku_count: i64,
        up_count: i64,
        status: &str,
    ) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE snapshot_log
             SET video_count   = ?1,
                 comment_count = ?2,
                 danmaku_count = ?3,
                 up_count      = ?4,
                 status        = ?5,
                 completed_at  = ?6
             WHERE snapshot_id = ?7",
            params![video_count, comment_count, danmaku_count, up_count, status, now, snapshot_id],
        )?;
        Ok(())
    }

    /// Return the most recent snapshot id that completed successfully.
    pub fn get_last_snapshot(&self) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT snapshot_id FROM snapshot_log
             WHERE status = 'ok'
             ORDER BY rowid DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check whether a full successful crawl already happened today
    /// (`snapshot_id` starts with `today_date` and `status = 'ok'`).
    pub fn has_full_crawl_today(&self, today_date: &str) -> Result<bool> {
        let pattern = format!("{}%", today_date);
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM snapshot_log
             WHERE snapshot_id LIKE ?1 AND status = 'ok'",
            params![pattern],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Helper: build a `StateManager` backed by an in-memory database.
    fn mem_state() -> StateManager {
        let conn = Connection::open_in_memory().expect("open_in_memory");
        StateManager::from_conn(conn).expect("from_conn")
    }

    // ── Table creation ────────────────────────────────────────────────

    #[test]
    fn test_create_tables() {
        let sm = mem_state();
        let tables: Vec<String> = {
            let mut stmt = sm
                .conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert!(tables.contains(&"collected_videos".to_string()));
        assert!(tables.contains(&"collected_comments".to_string()));
        assert!(tables.contains(&"collected_danmaku".to_string()));
        assert!(tables.contains(&"collected_ups".to_string()));
        assert!(tables.contains(&"failed_ups".to_string()));
        assert!(tables.contains(&"snapshot_log".to_string()));
    }

    // ── Video lifecycle ───────────────────────────────────────────────

    #[test]
    fn test_video_lifecycle() {
        let sm = mem_state();

        // Insert a fresh video — should return true (new).
        let is_new = sm
            .insert_video("BV1xx", "Title", "Tech", "Rust", 100, 200, 300, 400, "UP1", "snap1")
            .unwrap();
        assert!(is_new, "first insert should be new");

        // Re-insert same bvid — should return false (duplicate).
        let is_dup = sm
            .insert_video("BV1xx", "Title", "Tech", "Rust", 100, 200, 300, 400, "UP1", "snap1")
            .unwrap();
        assert!(!is_dup, "second insert should be a duplicate");

        // Not done yet.
        assert!(!sm.video_done("BV1xx").unwrap());

        // Mark detail.
        sm.mark_detail_done("BV1xx").unwrap();
        assert!(!sm.video_done("BV1xx").unwrap());

        // Mark comments.
        sm.mark_comments_done("BV1xx").unwrap();
        assert!(!sm.video_done("BV1xx").unwrap());

        // Mark danmaku — now fully done.
        sm.mark_danmaku_done("BV1xx").unwrap();
        assert!(sm.video_done("BV1xx").unwrap());

        // Fail twice.
        let count1 = sm.increment_fail("BV1xx", "err1").unwrap();
        assert_eq!(count1, 1);
        let count2 = sm.increment_fail("BV1xx", "err2").unwrap();
        assert_eq!(count2, 2);

        // Verify via get_video_info.
        let row = sm.get_video_info("BV1xx").unwrap().unwrap();
        assert_eq!(row.bvid, "BV1xx");
        assert_eq!(row.aid, 100);
        assert_eq!(row.cid, 200);
        assert!(row.has_detail);
        assert!(row.has_comments);
        assert!(row.has_danmaku);
        assert_eq!(row.fail_count, 2);

        // Reset failures.
        sm.reset_fail("BV1xx").unwrap();
        let row = sm.get_video_info("BV1xx").unwrap().unwrap();
        assert_eq!(row.fail_count, 0);

        // Non-existent bvid.
        assert!(sm.get_video_info("BV_DOES_NOT_EXIST").unwrap().is_none());
    }

    // ── get_failed_bvids / get_new_bvids_for_check ────────────────────

    #[test]
    fn test_failed_and_new_bvids() {
        let sm = mem_state();

        sm.insert_video("BV_A", "A", "", "", 1, 1, 0, 0, "", "").unwrap();
        sm.insert_video("BV_B", "B", "", "", 2, 2, 0, 0, "", "").unwrap();
        sm.insert_video("BV_C", "C", "", "", 3, 3, 0, 0, "", "").unwrap();

        // All should be "failed" (detail/comments/danmaku not done).
        let failed = sm.get_failed_bvids(10).unwrap();
        assert_eq!(failed.len(), 3);

        // Mark BV_A fully done.
        sm.mark_detail_done("BV_A").unwrap();
        sm.mark_comments_done("BV_A").unwrap();
        sm.mark_danmaku_done("BV_A").unwrap();

        let failed = sm.get_failed_bvids(10).unwrap();
        assert_eq!(failed.len(), 2);
        assert!(!failed.contains(&"BV_A".to_string()));

        // BV_B fails 5 times — still included if max_fail > 5.
        for i in 0..5 {
            sm.increment_fail("BV_B", &format!("err{i}")).unwrap();
        }
        let failed = sm.get_failed_bvids(10).unwrap();
        assert!(failed.contains(&"BV_B".to_string()));
        // But excluded if max_fail <= 5.
        let failed = sm.get_failed_bvids(5).unwrap();
        assert!(!failed.contains(&"BV_B".to_string()));

        // get_new_bvids_for_check
        let new = sm
            .get_new_bvids_for_check(&[
                "BV_A".to_string(),
                "BV_B".to_string(),
                "BV_NEW".to_string(),
            ])
            .unwrap();
        assert_eq!(new, vec!["BV_NEW".to_string()]);

        // Empty input.
        let new = sm.get_new_bvids_for_check(&[]).unwrap();
        assert!(new.is_empty());
    }

    // ── Comments ──────────────────────────────────────────────────────

    #[test]
    fn test_comments() {
        let sm = mem_state();

        assert!(!sm.is_comment_collected(42).unwrap());

        sm.insert_comments_batch(&[(42, "BV1"), (43, "BV1"), (99, "BV2")]).unwrap();

        assert!(sm.is_comment_collected(42).unwrap());
        assert!(sm.is_comment_collected(99).unwrap());
        assert!(!sm.is_comment_collected(1000).unwrap());

        let ids = sm.get_collected_comment_ids("BV1").unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&42));
        assert!(ids.contains(&43));
    }

    // ── Danmaku ───────────────────────────────────────────────────────

    #[test]
    fn test_danmaku() {
        let sm = mem_state();

        assert!(!sm.is_danmaku_collected(100).unwrap());

        sm.insert_danmaku_batch(&[(100, "BV1"), (101, "BV1"), (200, "BV2")]).unwrap();

        assert!(sm.is_danmaku_collected(100).unwrap());
        assert!(sm.is_danmaku_collected(200).unwrap());
        assert!(!sm.is_danmaku_collected(999).unwrap());

        let ids = sm.get_collected_danmaku_ids("BV1").unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&100));
        assert!(ids.contains(&101));
    }

    // ── UPs ───────────────────────────────────────────────────────────

    #[test]
    fn test_ups() {
        let sm = mem_state();

        // Not collected initially.
        assert!(!sm.is_up_collected("UP1").unwrap());
        assert!(!sm.should_skip_up("UP1").unwrap());

        // Collect.
        sm.mark_up_collected("UP1").unwrap();
        assert!(sm.is_up_collected("UP1").unwrap());

        // Record 3 failures — should now be skipped.
        sm.record_up_failure("UP1", "err1").unwrap();
        assert!(!sm.should_skip_up("UP1").unwrap());
        sm.record_up_failure("UP1", "err2").unwrap();
        assert!(!sm.should_skip_up("UP1").unwrap());
        sm.record_up_failure("UP1", "err3").unwrap();
        assert!(sm.should_skip_up("UP1").unwrap());
    }

    // ── Snapshots ─────────────────────────────────────────────────────

    #[test]
    fn test_snapshots() {
        let sm = mem_state();

        // No snapshot yet.
        assert!(sm.get_last_snapshot().unwrap().is_none());
        assert!(!sm.has_full_crawl_today("2025-01-01").unwrap());

        // Create and complete.
        sm.create_snapshot("2025-01-01_120000").unwrap();
        sm.complete_snapshot("2025-01-01_120000", 10, 50, 100, 5, "ok").unwrap();

        let last = sm.get_last_snapshot().unwrap();
        assert_eq!(last.as_deref(), Some("2025-01-01_120000"));

        assert!(sm.has_full_crawl_today("2025-01-01").unwrap());
        assert!(!sm.has_full_crawl_today("2025-01-02").unwrap());

        // A second, later snapshot should take precedence.
        sm.create_snapshot("2025-01-02_080000").unwrap();
        sm.complete_snapshot("2025-01-02_080000", 20, 80, 200, 8, "ok").unwrap();
        let last = sm.get_last_snapshot().unwrap();
        assert_eq!(last.as_deref(), Some("2025-01-02_080000"));

        // A failed snapshot should not count.
        sm.create_snapshot("2025-01-03_090000").unwrap();
        sm.complete_snapshot("2025-01-03_090000", 0, 0, 0, 0, "failed").unwrap();
        let last = sm.get_last_snapshot().unwrap();
        assert_eq!(last.as_deref(), Some("2025-01-02_080000"));
        assert!(!sm.has_full_crawl_today("2025-01-03").unwrap());
    }
}

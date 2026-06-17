use std::sync::Mutex;

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

/// Describes which sub-tasks a video still needs.
#[derive(Debug, Clone)]
pub struct VideoTasksNeeded {
    pub needs_detail: bool,
    pub needs_up: bool,
    pub needs_comments: bool,
    pub needs_danmaku: bool,
    pub aid: i64,
    pub cid: i64,
    pub danmaku_count: i64,
}

/// SQLite-backed state manager for crawler dedup / retry tracking.
///
/// Uses `Mutex<Connection>` for thread-safe access from multiple tokio tasks.
pub struct StateManager {
    conn: Mutex<Connection>,
}

impl StateManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let sm = Self {
            conn: Mutex::new(conn),
        };
        sm.create_tables()?;
        sm.migrate()?;
        Ok(sm)
    }

    /// Run schema migrations for columns added after the initial schema.
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Add last_danmaku_seg if missing (existing DBs won't have it)
        let _ = conn.execute(
            "ALTER TABLE collected_videos ADD COLUMN last_danmaku_seg INTEGER DEFAULT 0",
            [],
        );
        // Add last_snapshot_date if missing
        let _ = conn.execute(
            "ALTER TABLE collected_videos ADD COLUMN last_snapshot_date TEXT DEFAULT ''",
            [],
        );
        Ok(())
    }

    #[allow(dead_code)]
    fn from_conn(conn: Connection) -> Result<Self> {
        let sm = Self {
            conn: Mutex::new(conn),
        };
        sm.create_tables()?;
        Ok(sm)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS collected_videos (
                bvid                TEXT PRIMARY KEY,
                aid                 INTEGER DEFAULT 0,
                cid                 INTEGER DEFAULT 0,
                title               TEXT    DEFAULT '',
                category            TEXT    DEFAULT '',
                sub_category        TEXT    DEFAULT '',
                duration            INTEGER DEFAULT 0,
                danmaku_count       INTEGER DEFAULT 0,
                up_id               TEXT    DEFAULT '',
                has_detail          INTEGER DEFAULT 0,
                has_comments        INTEGER DEFAULT 0,
                has_danmaku         INTEGER DEFAULT 0,
                last_danmaku_seg    INTEGER DEFAULT 0,
                fail_count          INTEGER DEFAULT 0,
                last_error          TEXT,
                last_attempt        INTEGER,
                first_seen          DATE,
                created_at          INTEGER,
                updated_at          INTEGER
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

    #[allow(clippy::too_many_arguments)]
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
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
                now
            ],
        )?;
        Ok(conn.changes() > 0)
    }

    pub fn mark_detail_done(&self, bvid: &str, snapshot_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collected_videos SET has_detail = 1, updated_at = ?1, last_snapshot_date = ?2 WHERE bvid = ?3",
            params![now, snapshot_id, bvid],
        )?;
        Ok(())
    }

    pub fn mark_comments_done(&self, bvid: &str, snapshot_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collected_videos SET has_comments = 1, updated_at = ?1, last_snapshot_date = ?2 WHERE bvid = ?3",
            params![now, snapshot_id, bvid],
        )?;
        Ok(())
    }

    pub fn mark_danmaku_done(&self, bvid: &str, snapshot_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collected_videos SET has_danmaku = 1, updated_at = ?1, last_snapshot_date = ?2 WHERE bvid = ?3",
            params![now, snapshot_id, bvid],
        )?;
        Ok(())
    }

    /// Record the last successfully fetched danmaku segment for resume support.
    pub fn update_danmaku_seg(&self, bvid: &str, seg: u32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collected_videos SET last_danmaku_seg = ?1 WHERE bvid = ?2",
            params![seg as i64, bvid],
        )?;
        Ok(())
    }

    /// Get the last successfully fetched danmaku segment (0 = none yet).
    pub fn get_last_danmaku_seg(&self, bvid: &str) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let seg: i64 = conn.query_row(
            "SELECT last_danmaku_seg FROM collected_videos WHERE bvid = ?1",
            params![bvid],
            |row| row.get(0),
        )?;
        Ok(seg as u32)
    }

    pub fn increment_fail(&self, bvid: &str, error: &str) -> Result<u32> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collected_videos
             SET fail_count = fail_count + 1, last_error = ?1, last_attempt = ?2, updated_at = ?2
             WHERE bvid = ?3",
            params![error, now, bvid],
        )?;
        let count: u32 = conn.query_row(
            "SELECT fail_count FROM collected_videos WHERE bvid = ?1",
            params![bvid],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn reset_fail(&self, bvid: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE collected_videos SET fail_count = 0, last_error = NULL, updated_at = ?1 WHERE bvid = ?2",
            params![now, bvid],
        )?;
        Ok(())
    }

    pub fn get_video_info(&self, bvid: &str) -> Result<Option<VideoRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
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

    pub fn get_failed_bvids(&self, max_fail: u32) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT bvid FROM collected_videos
             WHERE (has_detail = 0 OR has_comments = 0 OR has_danmaku = 0) AND fail_count < ?1",
        )?;
        let rows = stmt.query_map(params![max_fail], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn get_new_bvids_for_check(&self, bvids: &[String]) -> Result<Vec<String>> {
        if bvids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<String> = (0..bvids.len()).map(|i| format!("?{}", i + 1)).collect();
        let sql = format!(
            "SELECT bvid FROM collected_videos WHERE bvid IN ({})",
            placeholders.join(", ")
        );
        let known: std::collections::HashSet<String> = {
            let mut stmt = conn.prepare(&sql)?;
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            for b in bvids {
                params_vec.push(Box::new(b.clone()));
            }
            let refs: Vec<&dyn rusqlite::types::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
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

    pub fn video_done(&self, bvid: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let done: bool = conn.query_row(
            "SELECT (v.has_detail = 1 AND v.has_comments = 1 AND v.has_danmaku = 1
                     AND EXISTS (SELECT 1 FROM collected_ups u WHERE u.up_id = v.up_id))
             FROM collected_videos v WHERE v.bvid = ?1",
            params![bvid],
            |row| row.get(0),
        )?;
        Ok(done)
    }

    /// Determine which sub-tasks a video still needs.
    pub fn get_video_tasks_needed(&self, bvid: &str, up_id: &str) -> Result<VideoTasksNeeded> {
        let conn = self.conn.lock().unwrap();
        let row = conn.query_row(
            "SELECT has_detail, has_comments, has_danmaku, aid, cid, danmaku_count FROM collected_videos WHERE bvid = ?1",
            params![bvid],
            |row| {
                Ok((
                    row.get::<_, i32>(0)? != 0,
                    row.get::<_, i32>(1)? != 0,
                    row.get::<_, i32>(2)? != 0,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )?;

        let up_collected: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_ups WHERE up_id = ?1",
            params![up_id],
            |row| row.get(0),
        )?;

        Ok(VideoTasksNeeded {
            needs_detail: !row.0,
            needs_up: !up_collected,
            needs_comments: !row.1,
            needs_danmaku: !row.2,
            aid: row.3,
            cid: row.4,
            danmaku_count: row.5,
        })
    }

    /// Reset in-flight sub-task flags after a crash.
    ///
    /// Only resets comments/danmaku for videos where `has_detail = 0` (detail was
    /// never completed, so comments/danmaku data is unreliable). Videos with detail
    /// already done keep their comments/danmaku flags — the data files are valid
    /// and the task loop's dedup handles re-fetched entries.
    ///
    /// UP info is tracked in `collected_ups` and unaffected by this reset.
    /// Reset collection flags for ranked videos that haven't been crawled for this snapshot.
    /// This ensures full daily crawl while supporting resume-after-crash:
    /// videos already crawled for this snapshot keep their flags.
    pub fn reset_flags_for_stale_videos(&self, bvids: &[String], snapshot_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "UPDATE collected_videos SET has_detail = 0, has_comments = 0, has_danmaku = 0
             WHERE bvid = ?1 AND (last_snapshot_date IS NULL OR last_snapshot_date != ?2)"
        )?;
        let mut reset_count = 0;
        for bvid in bvids {
            let rows = stmt.execute(params![bvid, snapshot_id])?;
            reset_count += rows;
        }
        tracing::info!(total = bvids.len(), reset = reset_count, "reset flags for stale videos");
        Ok(())
    }

    // ── Comment methods ───────────────────────────────────────────────

    pub fn is_comment_collected(&self, comment_id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_comments WHERE comment_id = ?1",
            params![comment_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    pub fn insert_comments_batch(&self, ids: &[(i64, &str)]) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO collected_comments (comment_id, bvid, collect_time) VALUES (?1, ?2, ?3)",
        )?;
        for (cid, bvid) in ids {
            stmt.execute(params![cid, bvid, now])?;
        }
        Ok(())
    }

    pub fn get_collected_comment_ids(&self, bvid: &str) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT comment_id FROM collected_comments WHERE bvid = ?1")?;
        let rows = stmt.query_map(params![bvid], |row| row.get::<_, i64>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ── Danmaku methods ───────────────────────────────────────────────

    pub fn is_danmaku_collected(&self, danmaku_id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_danmaku WHERE danmaku_id = ?1",
            params![danmaku_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    pub fn insert_danmaku_batch(&self, ids: &[(i64, &str)]) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO collected_danmaku (danmaku_id, bvid, collect_time) VALUES (?1, ?2, ?3)",
        )?;
        for (did, bvid) in ids {
            stmt.execute(params![did, bvid, now])?;
        }
        Ok(())
    }

    pub fn get_collected_danmaku_ids(&self, bvid: &str) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT danmaku_id FROM collected_danmaku WHERE bvid = ?1")?;
        let rows = stmt.query_map(params![bvid], |row| row.get::<_, i64>(0))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // ── UP methods ────────────────────────────────────────────────────

    pub fn is_up_collected(&self, up_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM collected_ups WHERE up_id = ?1",
            params![up_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    pub fn mark_up_collected(&self, up_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO collected_ups (up_id, collect_time) VALUES (?1, ?2)",
            params![up_id, now],
        )?;
        Ok(())
    }

    pub fn should_skip_up(&self, up_id: &str) -> Result<bool> {
        let now = Utc::now().timestamp();
        let day_ago = now - 86_400;
        let conn = self.conn.lock().unwrap();
        let skip: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM failed_ups
             WHERE up_id = ?1 AND retry_count >= 3 AND last_attempt > ?2",
            params![up_id, day_ago],
            |row| row.get(0),
        )?;
        Ok(skip)
    }

    pub fn record_up_failure(&self, up_id: &str, error: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO failed_ups (up_id, retry_count, last_error, last_attempt)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(up_id) DO UPDATE SET
                 retry_count = retry_count + 1, last_error = excluded.last_error, last_attempt = excluded.last_attempt",
            params![up_id, error, now],
        )?;
        Ok(())
    }

    // ── Snapshot methods ──────────────────────────────────────────────

    pub fn create_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO snapshot_log (snapshot_id, started_at) VALUES (?1, ?2)",
            params![snapshot_id, now],
        )?;
        Ok(())
    }

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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE snapshot_log SET video_count=?1, comment_count=?2, danmaku_count=?3, up_count=?4, status=?5, completed_at=?6 WHERE snapshot_id=?7",
            params![video_count, comment_count, danmaku_count, up_count, status, now, snapshot_id],
        )?;
        Ok(())
    }

    pub fn get_last_snapshot(&self) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT snapshot_id FROM snapshot_log WHERE status = 'ok' ORDER BY rowid DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn has_full_crawl_today(&self, today_date: &str) -> Result<bool> {
        let pattern = format!("{}%", today_date);
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM snapshot_log WHERE snapshot_id LIKE ?1 AND status = 'ok'",
            params![pattern],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn mem_state() -> StateManager {
        let conn = Connection::open_in_memory().expect("open_in_memory");
        StateManager::from_conn(conn).expect("from_conn")
    }

    #[test]
    fn test_video_lifecycle() {
        let sm = mem_state();
        let is_new = sm
            .insert_video(
                "BV1xx", "Title", "Tech", "Rust", 100, 200, 300, 400, "UP1", "snap1",
            )
            .unwrap();
        assert!(is_new);
        let is_dup = sm
            .insert_video(
                "BV1xx", "Title", "Tech", "Rust", 100, 200, 300, 400, "UP1", "snap1",
            )
            .unwrap();
        assert!(!is_dup);
        assert!(!sm.video_done("BV1xx").unwrap());
        sm.mark_detail_done("BV1xx").unwrap();
        sm.mark_comments_done("BV1xx").unwrap();
        sm.mark_danmaku_done("BV1xx").unwrap();
        // Still not done — UP not collected
        assert!(!sm.video_done("BV1xx").unwrap());
        sm.mark_up_collected("UP1").unwrap();
        assert!(sm.video_done("BV1xx").unwrap());
    }

    #[test]
    fn test_get_video_tasks_needed() {
        let sm = mem_state();
        sm.insert_video("BV1", "T", "", "", 1, 1, 0, 0, "UP1", "")
            .unwrap();

        let tasks = sm.get_video_tasks_needed("BV1", "UP1").unwrap();
        assert!(tasks.needs_detail);
        assert!(tasks.needs_up);
        assert!(tasks.needs_comments);
        assert!(tasks.needs_danmaku);

        sm.mark_detail_done("BV1").unwrap();
        sm.mark_up_collected("UP1").unwrap();
        let tasks = sm.get_video_tasks_needed("BV1", "UP1").unwrap();
        assert!(!tasks.needs_detail);
        assert!(!tasks.needs_up);
        assert!(tasks.needs_comments);
    }

    #[test]
    fn test_reset_incomplete_flags() {
        let sm = mem_state();
        sm.insert_video("BV1", "", "", "", 1, 1, 0, 0, "UP1", "")
            .unwrap();
        sm.insert_video("BV2", "", "", "", 2, 2, 0, 0, "UP2", "")
            .unwrap();
        sm.insert_video("BV3", "", "", "", 3, 3, 0, 0, "UP3", "")
            .unwrap();

        // BV1: detail+comments+danmaku done
        sm.mark_detail_done("BV1").unwrap();
        sm.mark_comments_done("BV1").unwrap();
        sm.mark_danmaku_done("BV1").unwrap();

        // BV2: detail done, comments done, danmaku NOT done
        sm.mark_detail_done("BV2").unwrap();
        sm.mark_comments_done("BV2").unwrap();

        // BV3: nothing done

        sm.reset_incomplete_flags().unwrap();

        // BV1: detail done → comments/danmaku preserved
        let row1 = sm.get_video_info("BV1").unwrap().unwrap();
        assert!(row1.has_comments);
        assert!(row1.has_danmaku);

        // BV2: detail done → comments preserved, danmaku stays 0
        let row2 = sm.get_video_info("BV2").unwrap().unwrap();
        assert!(row2.has_comments);
        assert!(!row2.has_danmaku);

        // BV3: detail not done → flags stay 0 (reset is no-op for already-0)
        let row3 = sm.get_video_info("BV3").unwrap().unwrap();
        assert!(!row3.has_comments);
        assert!(!row3.has_danmaku);
    }

    #[test]
    fn test_video_done_requires_up() {
        let sm = mem_state();
        sm.insert_video("BV1", "T", "", "", 1, 1, 0, 0, "UP1", "")
            .unwrap();

        // All sub-tasks done but UP not collected
        sm.mark_detail_done("BV1").unwrap();
        sm.mark_comments_done("BV1").unwrap();
        sm.mark_danmaku_done("BV1").unwrap();
        assert!(!sm.video_done("BV1").unwrap());

        // After UP collected, video is done
        sm.mark_up_collected("UP1").unwrap();
        assert!(sm.video_done("BV1").unwrap());

        // On restart, video stays done — UP task won't be generated
        sm.reset_incomplete_flags().unwrap();
        assert!(sm.video_done("BV1").unwrap());
    }

    #[test]
    fn test_video_done_up_already_collected_for_another_video() {
        let sm = mem_state();
        // UP1 was collected via another video
        sm.mark_up_collected("UP1").unwrap();

        sm.insert_video("BV1", "T", "", "", 1, 1, 0, 0, "UP1", "")
            .unwrap();
        sm.mark_detail_done("BV1").unwrap();
        sm.mark_comments_done("BV1").unwrap();
        sm.mark_danmaku_done("BV1").unwrap();
        // UP already collected → video done immediately
        assert!(sm.video_done("BV1").unwrap());
    }

    #[test]
    fn test_comments_and_danmaku() {
        let sm = mem_state();
        sm.insert_comments_batch(&[(42, "BV1"), (43, "BV1")])
            .unwrap();
        assert!(sm.is_comment_collected(42).unwrap());
        let ids = sm.get_collected_comment_ids("BV1").unwrap();
        assert_eq!(ids.len(), 2);

        sm.insert_danmaku_batch(&[(100, "BV1")]).unwrap();
        assert!(sm.is_danmaku_collected(100).unwrap());
    }

    #[test]
    fn test_snapshots() {
        let sm = mem_state();
        assert!(sm.get_last_snapshot().unwrap().is_none());
        sm.create_snapshot("2025-01-01_120000").unwrap();
        sm.complete_snapshot("2025-01-01_120000", 10, 50, 100, 5, "ok")
            .unwrap();
        assert_eq!(
            sm.get_last_snapshot().unwrap().as_deref(),
            Some("2025-01-01_120000")
        );
        assert!(sm.has_full_crawl_today("2025-01-01").unwrap());
    }
}

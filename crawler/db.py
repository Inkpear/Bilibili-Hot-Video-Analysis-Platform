"""
SQLite 状态管理：已采集去重 + 评论/弹幕队列 + 快照日志。
"""

import logging
import sqlite3
import time

from config import DB_PATH

logger = logging.getLogger("crawler")

CREATE_SQL = """
CREATE TABLE IF NOT EXISTS collected_videos (
    bvid          TEXT PRIMARY KEY,
    title         TEXT,
    category      TEXT,
    sub_category  TEXT,
    aid           INTEGER,
    cid           INTEGER,
    duration      INTEGER,
    danmaku_count INTEGER DEFAULT 0,
    has_detail    INTEGER DEFAULT 0,
    has_comments  INTEGER DEFAULT 0,
    has_danmaku   INTEGER DEFAULT 0,
    first_seen_snapshot TEXT,
    collect_time  INTEGER
);

CREATE TABLE IF NOT EXISTS collected_comments (
    comment_id    INTEGER PRIMARY KEY,
    bvid          TEXT,
    collect_time  INTEGER
);

CREATE TABLE IF NOT EXISTS collected_danmaku (
    danmaku_id    INTEGER PRIMARY KEY,
    bvid          TEXT,
    collect_time  INTEGER
);

CREATE TABLE IF NOT EXISTS collected_ups (
    up_id         TEXT PRIMARY KEY,
    collect_time  INTEGER
);

CREATE TABLE IF NOT EXISTS failed_ups (
    up_id         TEXT PRIMARY KEY,
    retry_count   INTEGER DEFAULT 0,
    last_error    TEXT,
    updated_at    INTEGER
);

CREATE TABLE IF NOT EXISTS queue_comments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    bvid          TEXT NOT NULL,
    aid           INTEGER NOT NULL,
    priority      INTEGER DEFAULT 0,
    retry_count   INTEGER DEFAULT 0,
    status        TEXT DEFAULT 'pending',
    created_at    INTEGER,
    updated_at    INTEGER
);

CREATE TABLE IF NOT EXISTS queue_danmaku (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    bvid          TEXT NOT NULL,
    cid           INTEGER NOT NULL,
    priority      INTEGER DEFAULT 0,
    retry_count   INTEGER DEFAULT 0,
    status        TEXT DEFAULT 'pending',
    created_at    INTEGER,
    updated_at    INTEGER
);

CREATE TABLE IF NOT EXISTS snapshot_log (
    snapshot_id       TEXT PRIMARY KEY,
    start_time        INTEGER,
    end_time          INTEGER,
    rank_count        INTEGER DEFAULT 0,
    new_video_count   INTEGER DEFAULT 0,
    comments_processed INTEGER DEFAULT 0,
    danmaku_processed  INTEGER DEFAULT 0,
    status            TEXT DEFAULT 'pending'
);

CREATE INDEX IF NOT EXISTS idx_queue_comments_status ON queue_comments(status);
CREATE INDEX IF NOT EXISTS idx_queue_danmaku_status ON queue_danmaku(status);
CREATE INDEX IF NOT EXISTS idx_collected_videos_has ON collected_videos(has_detail, has_comments, has_danmaku);
CREATE INDEX IF NOT EXISTS idx_failed_ups_retry ON failed_ups(retry_count, updated_at);
"""

MAX_RETRIES = 3


def _migrate_add_column(conn, table, column_def):
    """幂等迁移：检查列是否存在，不存在则添加。"""
    try:
        conn.execute(f"ALTER TABLE {table} ADD COLUMN {column_def}")
    except sqlite3.OperationalError as e:
        if "duplicate column" in str(e).lower():
            pass
        else:
            raise


class CrawlerDB:
    def __init__(self, db_path=DB_PATH):
        self.db_path = db_path
        self._conn = sqlite3.connect(db_path, check_same_thread=False)
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA foreign_keys=ON")
        self._conn.executescript(CREATE_SQL)
        self._conn.commit()

        # 迁移：旧表可能缺少 retry_count 列
        _migrate_add_column(self._conn, "queue_comments", "retry_count INTEGER DEFAULT 0")
        _migrate_add_column(self._conn, "queue_danmaku", "retry_count INTEGER DEFAULT 0")
        _migrate_add_column(self._conn, "collected_videos", "danmaku_count INTEGER DEFAULT 0")

        # 重置启动前残留的 processing 项
        self.reset_stale_processing()

    def close(self):
        self._conn.close()

    # ------------------------------------------------------------------
    # 启动恢复
    # ------------------------------------------------------------------

    def reset_stale_processing(self, timeout_seconds=1800):
        """将超时的 processing 项重置为 pending。启动时自动调用。"""
        cutoff = int(time.time()) - timeout_seconds
        for table in ("queue_comments", "queue_danmaku"):
            rows = self._conn.execute(
                f"UPDATE {table} SET status='pending', updated_at=? "
                f"WHERE status='processing' AND updated_at < ?",
                (int(time.time()), cutoff)
            ).rowcount
            if rows:
                logger.info(f"  恢复 {rows} 个残留的 {table} 任务 (status: processing → pending)")
        self._conn.commit()

    # ------------------------------------------------------------------
    # 视频
    # ------------------------------------------------------------------

    def is_video_collected(self, bvid: str) -> bool:
        row = self._conn.execute(
            "SELECT 1 FROM collected_videos WHERE bvid=?", (bvid,)
        ).fetchone()
        return row is not None

    def add_video(self, bvid, title="", category="", sub_category="",
                  aid=0, cid=0, duration=0, danmaku_count=0, snapshot_id="", ts=None):
        if ts is None:
            ts = int(time.time())
        self._conn.execute(
            """INSERT OR IGNORE INTO collected_videos
               (bvid, title, category, sub_category, aid, cid, duration, danmaku_count,
                first_seen_snapshot, collect_time)
               VALUES (?,?,?,?,?,?,?,?,?,?)""",
            (bvid, title, category, sub_category, aid, cid, duration, danmaku_count,
             snapshot_id, ts)
        )
        self._conn.commit()

    def mark_detail_done(self, bvid):
        self._conn.execute(
            "UPDATE collected_videos SET has_detail=1 WHERE bvid=?", (bvid,)
        )
        self._conn.commit()

    def mark_comments_done(self, bvid):
        self._conn.execute(
            "UPDATE collected_videos SET has_comments=1 WHERE bvid=?", (bvid,)
        )
        self._conn.commit()

    def mark_danmaku_done(self, bvid):
        self._conn.execute(
            "UPDATE collected_videos SET has_danmaku=1 WHERE bvid=?", (bvid,)
        )
        self._conn.commit()

    def get_videos_missing_detail(self, exclude_bvids=None):
        exclude_bvids = set(exclude_bvids or [])
        rows = self._conn.execute(
            "SELECT bvid FROM collected_videos WHERE has_detail=0 ORDER BY collect_time ASC"
        ).fetchall()
        if exclude_bvids:
            return [r[0] for r in rows if r[0] not in exclude_bvids]
        return [r[0] for r in rows]

    def get_video_count(self):
        row = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos"
        ).fetchone()
        return row[0]

    def get_video_info(self, bvid):
        row = self._conn.execute(
            "SELECT bvid,title,category,aid,cid,duration,danmaku_count,"
            "has_detail,has_comments,has_danmaku "
            "FROM collected_videos WHERE bvid=?",
            (bvid,)
        ).fetchone()
        if row is None:
            return None
        return {
            "bvid": row[0], "title": row[1], "category": row[2],
            "aid": row[3], "cid": row[4], "duration": row[5],
            "danmaku_count": row[6],
            "has_detail": row[7], "has_comments": row[8], "has_danmaku": row[9],
        }

    # ------------------------------------------------------------------
    # UP 主
    # ------------------------------------------------------------------

    def is_up_collected(self, up_id: str) -> bool:
        row = self._conn.execute(
            "SELECT 1 FROM collected_ups WHERE up_id=?", (up_id,)
        ).fetchone()
        return row is not None

    def add_up(self, up_id, ts=None):
        if ts is None:
            ts = int(time.time())
        self._conn.execute(
            "INSERT OR IGNORE INTO collected_ups (up_id, collect_time) VALUES (?,?)",
            (up_id, ts)
        )
        self._conn.execute("DELETE FROM failed_ups WHERE up_id=?", (up_id,))
        self._conn.commit()

    def should_skip_up(self, up_id: str, max_retries=3, cooldown_seconds=86400) -> bool:
        row = self._conn.execute(
            "SELECT retry_count, updated_at FROM failed_ups WHERE up_id=?", (up_id,)
        ).fetchone()
        if not row:
            return False
        retry_count, updated_at = row
        if retry_count < max_retries:
            return False
        return updated_at and updated_at > int(time.time()) - cooldown_seconds

    def record_up_failure(self, up_id: str, last_error=""):
        ts = int(time.time())
        row = self._conn.execute(
            "SELECT retry_count FROM failed_ups WHERE up_id=?", (up_id,)
        ).fetchone()
        retry_count = (row[0] if row else 0) + 1
        if row:
            self._conn.execute(
                "UPDATE failed_ups SET retry_count=?, last_error=?, updated_at=? WHERE up_id=?",
                (retry_count, last_error, ts, up_id)
            )
        else:
            self._conn.execute(
                "INSERT INTO failed_ups (up_id, retry_count, last_error, updated_at) VALUES (?,?,?,?)",
                (up_id, retry_count, last_error, ts)
            )
        self._conn.commit()
        return retry_count

    # ------------------------------------------------------------------
    # 评论
    # ------------------------------------------------------------------

    def is_comment_collected(self, comment_id: int) -> bool:
        row = self._conn.execute(
            "SELECT 1 FROM collected_comments WHERE comment_id=?", (comment_id,)
        ).fetchone()
        return row is not None

    def add_comment_ids_batch(self, ids: list[tuple[int, str]]):
        ts = int(time.time())
        self._conn.executemany(
            "INSERT OR IGNORE INTO collected_comments (comment_id, bvid, collect_time) VALUES (?,?,?)",
            [(cid, bvid, ts) for cid, bvid in ids]
        )
        self._conn.commit()

    def get_collected_comment_ids_for_bvid(self, bvid: str) -> set[int]:
        rows = self._conn.execute(
            "SELECT comment_id FROM collected_comments WHERE bvid=?", (bvid,)
        ).fetchall()
        return {r[0] for r in rows}

    # ------------------------------------------------------------------
    # 弹幕
    # ------------------------------------------------------------------

    def is_danmaku_collected(self, danmaku_id: int) -> bool:
        row = self._conn.execute(
            "SELECT 1 FROM collected_danmaku WHERE danmaku_id=?", (danmaku_id,)
        ).fetchone()
        return row is not None

    def get_collected_danmaku_ids_for_bvid(self, bvid: str) -> set[int]:
        rows = self._conn.execute(
            "SELECT danmaku_id FROM collected_danmaku WHERE bvid=?", (bvid,)
        ).fetchall()
        return {r[0] for r in rows}

    def add_danmaku_ids_batch(self, ids: list[tuple[int, str]]):
        ts = int(time.time())
        self._conn.executemany(
            "INSERT OR IGNORE INTO collected_danmaku (danmaku_id, bvid, collect_time) VALUES (?,?,?)",
            [(did, bvid, ts) for did, bvid in ids]
        )
        self._conn.commit()

    # ------------------------------------------------------------------
    # 评论队列
    # ------------------------------------------------------------------

    def enqueue_comment(self, bvid, aid, priority=0):
        existing = self._conn.execute(
            "SELECT 1 FROM queue_comments WHERE bvid=? AND status IN ('pending','processing')",
            (bvid,)
        ).fetchone()
        if existing:
            return
        ts = int(time.time())
        self._conn.execute(
            "INSERT INTO queue_comments (bvid, aid, priority, created_at) VALUES (?,?,?,?)",
            (bvid, aid, priority, ts)
        )
        self._conn.commit()

    def dequeue_comments(self, limit=10):
        rows = self._conn.execute(
            "SELECT id, bvid, aid FROM queue_comments WHERE status='pending' "
            "ORDER BY priority DESC, retry_count ASC, id ASC LIMIT ?",
            (limit,)
        ).fetchall()
        if not rows:
            return []
        ids = [r[0] for r in rows]
        ts = int(time.time())
        self._conn.execute(
            f"UPDATE queue_comments SET status='processing', updated_at=? "
            f"WHERE id IN ({','.join('?'*len(ids))})",
            [ts] + ids
        )
        self._conn.commit()
        return [(r[0], r[1], r[2]) for r in rows]

    def complete_comment(self, queue_id, success=True):
        """标记任务完成。success=False 时自动重试（最多 MAX_RETRIES 次）。"""
        ts = int(time.time())
        if success:
            self._conn.execute(
                "UPDATE queue_comments SET status='done', updated_at=? WHERE id=?",
                (ts, queue_id)
            )
        else:
            row = self._conn.execute(
                "SELECT retry_count FROM queue_comments WHERE id=?", (queue_id,)
            ).fetchone()
            count = (row[0] if row else 0) + 1
            if count >= MAX_RETRIES:
                self._conn.execute(
                    "UPDATE queue_comments SET status='failed', retry_count=?, updated_at=? WHERE id=?",
                    (count, ts, queue_id)
                )
                logger.warning(f"  评论任务 {queue_id} 重试 {count} 次后标记为 failed")
            else:
                self._conn.execute(
                    "UPDATE queue_comments SET status='pending', retry_count=?, updated_at=? WHERE id=?",
                    (count, ts, queue_id)
                )
                logger.info(f"  评论任务 {queue_id} 重试 {count}/{MAX_RETRIES}，放回队列")
        self._conn.commit()

    def comment_queue_size(self):
        row = self._conn.execute(
            "SELECT COUNT(*) FROM queue_comments WHERE status='pending'"
        ).fetchone()
        return row[0]

    def queue_status_counts(self, table):
        rows = self._conn.execute(
            f"SELECT status, COUNT(*) FROM {table} GROUP BY status"
        ).fetchall()
        return {status: count for status, count in rows}

    # ------------------------------------------------------------------
    # 弹幕队列
    # ------------------------------------------------------------------

    def enqueue_danmaku(self, bvid, cid, priority=0):
        existing = self._conn.execute(
            "SELECT 1 FROM queue_danmaku WHERE bvid=? AND status IN ('pending','processing')",
            (bvid,)
        ).fetchone()
        if existing:
            return
        ts = int(time.time())
        self._conn.execute(
            "INSERT INTO queue_danmaku (bvid, cid, priority, created_at) VALUES (?,?,?,?)",
            (bvid, cid, priority, ts)
        )
        self._conn.commit()

    def dequeue_danmaku(self, limit=15):
        rows = self._conn.execute(
            "SELECT id, bvid, cid FROM queue_danmaku WHERE status='pending' "
            "ORDER BY priority DESC, retry_count ASC, id ASC LIMIT ?",
            (limit,)
        ).fetchall()
        if not rows:
            return []
        ids = [r[0] for r in rows]
        ts = int(time.time())
        self._conn.execute(
            f"UPDATE queue_danmaku SET status='processing', updated_at=? "
            f"WHERE id IN ({','.join('?'*len(ids))})",
            [ts] + ids
        )
        self._conn.commit()
        return [(r[0], r[1], r[2]) for r in rows]

    def complete_danmaku(self, queue_id, success=True):
        ts = int(time.time())
        if success:
            self._conn.execute(
                "UPDATE queue_danmaku SET status='done', updated_at=? WHERE id=?",
                (ts, queue_id)
            )
        else:
            row = self._conn.execute(
                "SELECT retry_count FROM queue_danmaku WHERE id=?", (queue_id,)
            ).fetchone()
            count = (row[0] if row else 0) + 1
            if count >= MAX_RETRIES:
                self._conn.execute(
                    "UPDATE queue_danmaku SET status='failed', retry_count=?, updated_at=? WHERE id=?",
                    (count, ts, queue_id)
                )
                logger.warning(f"  弹幕任务 {queue_id} 重试 {count} 次后标记为 failed")
            else:
                self._conn.execute(
                    "UPDATE queue_danmaku SET status='pending', retry_count=?, updated_at=? WHERE id=?",
                    (count, ts, queue_id)
                )
                logger.info(f"  弹幕任务 {queue_id} 重试 {count}/{MAX_RETRIES}，放回队列")
        self._conn.commit()

    def danmaku_queue_size(self):
        row = self._conn.execute(
            "SELECT COUNT(*) FROM queue_danmaku WHERE status='pending'"
        ).fetchone()
        return row[0]

    # ------------------------------------------------------------------
    # 快照日志
    # ------------------------------------------------------------------

    def start_snapshot(self, snapshot_id):
        ts = int(time.time())
        self._conn.execute(
            "INSERT OR REPLACE INTO snapshot_log (snapshot_id, start_time, status) VALUES (?,?,'running')",
            (snapshot_id, ts)
        )
        self._conn.commit()

    def finish_snapshot(self, snapshot_id, rank_count=0, new_video_count=0,
                        comments_processed=0, danmaku_processed=0, status="ok"):
        ts = int(time.time())
        self._conn.execute(
            """UPDATE snapshot_log SET end_time=?, rank_count=?, new_video_count=?,
               comments_processed=?, danmaku_processed=?, status=?
               WHERE snapshot_id=?""",
            (ts, rank_count, new_video_count, comments_processed, danmaku_processed,
             status, snapshot_id)
        )
        self._conn.commit()

    def get_last_snapshot(self):
        row = self._conn.execute(
            "SELECT snapshot_id, end_time FROM snapshot_log WHERE status='ok' ORDER BY end_time DESC LIMIT 1"
        ).fetchone()
        return {"snapshot_id": row[0], "end_time": row[1]} if row else None

    # ------------------------------------------------------------------
    # 统计
    # ------------------------------------------------------------------

    def stats(self):
        comment_status = self.queue_status_counts("queue_comments")
        danmaku_status = self.queue_status_counts("queue_danmaku")
        return {
            "videos": self.get_video_count(),
            "comments": self._conn.execute("SELECT COUNT(*) FROM collected_comments").fetchone()[0],
            "danmaku": self._conn.execute("SELECT COUNT(*) FROM collected_danmaku").fetchone()[0],
            "ups": self._conn.execute("SELECT COUNT(*) FROM collected_ups").fetchone()[0],
            "comment_queue": self.comment_queue_size(),
            "danmaku_queue": self.danmaku_queue_size(),
            "comment_processing": comment_status.get("processing", 0),
            "comment_failed": comment_status.get("failed", 0),
            "danmaku_processing": danmaku_status.get("processing", 0),
            "danmaku_failed": danmaku_status.get("failed", 0),
            "up_failed": self._conn.execute("SELECT COUNT(*) FROM failed_ups").fetchone()[0],
        }

"""
SQLite 状态管理：已采集去重 + 失败追踪 + 快照日志。
"""

import logging
import sqlite3
import time

from config import DB_PATH, MAX_FAIL_COUNT

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
    fail_count    INTEGER DEFAULT 0,
    last_fail_time INTEGER,
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

CREATE TABLE IF NOT EXISTS snapshot_log (
    snapshot_id       TEXT PRIMARY KEY,
    start_time        INTEGER,
    end_time          INTEGER,
    rank_count        INTEGER DEFAULT 0,
    new_video_count   INTEGER DEFAULT 0,
    videos_processed  INTEGER DEFAULT 0,
    videos_ok         INTEGER DEFAULT 0,
    videos_failed     INTEGER DEFAULT 0,
    status            TEXT DEFAULT 'pending'
);

CREATE INDEX IF NOT EXISTS idx_collected_videos_status ON collected_videos(has_detail, has_comments, has_danmaku, fail_count);
CREATE INDEX IF NOT EXISTS idx_failed_ups_retry ON failed_ups(retry_count, updated_at);
"""


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

        # 迁移：旧表可能缺少列
        _migrate_add_column(self._conn, "collected_videos", "fail_count INTEGER DEFAULT 0")
        _migrate_add_column(self._conn, "collected_videos", "last_fail_time INTEGER")
        _migrate_add_column(self._conn, "collected_videos", "danmaku_count INTEGER DEFAULT 0")

    def close(self):
        self._conn.close()

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

    def reset_fail_count(self, bvid):
        self._conn.execute(
            "UPDATE collected_videos SET fail_count=0, last_fail_time=NULL WHERE bvid=?", (bvid,)
        )
        self._conn.commit()

    def increment_fail_count(self, bvid):
        ts = int(time.time())
        self._conn.execute(
            "UPDATE collected_videos SET fail_count = fail_count + 1, last_fail_time = ? WHERE bvid=?",
            (ts, bvid)
        )
        self._conn.commit()
        row = self._conn.execute(
            "SELECT fail_count FROM collected_videos WHERE bvid=?", (bvid,)
        ).fetchone()
        return row[0] if row else 0

    def get_video_info(self, bvid):
        row = self._conn.execute(
            "SELECT bvid,title,category,aid,cid,duration,danmaku_count,"
            "has_detail,has_comments,has_danmaku,fail_count "
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
            "fail_count": row[10],
        }

    def video_done(self, bvid):
        """返回 True 如果视频所有组件都已完成。"""
        info = self.get_video_info(bvid)
        if info is None:
            return False
        return bool(info["has_detail"] and info["has_comments"] and info["has_danmaku"])

    def video_abandoned(self, bvid):
        """返回 True 如果视频已永久放弃。"""
        info = self.get_video_info(bvid)
        if info is None:
            return False
        return info["fail_count"] >= MAX_FAIL_COUNT

    def get_videos_missing_detail(self, exclude_bvids=None):
        exclude_bvids = set(exclude_bvids or [])
        rows = self._conn.execute(
            "SELECT bvid FROM collected_videos WHERE has_detail=0 AND fail_count<? ORDER BY collect_time ASC",
            (MAX_FAIL_COUNT,)
        ).fetchall()
        return [r[0] for r in rows if r[0] not in exclude_bvids]

    def get_video_count(self):
        row = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos"
        ).fetchone()
        return row[0]

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
                        videos_processed=0, videos_ok=0, videos_failed=0, status="ok"):
        ts = int(time.time())
        self._conn.execute(
            """UPDATE snapshot_log SET end_time=?, rank_count=?, new_video_count=?,
               videos_processed=?, videos_ok=?, videos_failed=?, status=?
               WHERE snapshot_id=?""",
            (ts, rank_count, new_video_count, videos_processed, videos_ok, videos_failed,
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
        videos = self.get_video_count()
        ok = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos WHERE has_detail=1 AND has_comments=1 AND has_danmaku=1"
        ).fetchone()[0]
        fail = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos WHERE fail_count>=?", (MAX_FAIL_COUNT,)
        ).fetchone()[0]
        pending = videos - ok - fail
        has_detail = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos WHERE has_detail=1"
        ).fetchone()[0]
        has_comments = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos WHERE has_comments=1"
        ).fetchone()[0]
        has_danmaku = self._conn.execute(
            "SELECT COUNT(*) FROM collected_videos WHERE has_danmaku=1"
        ).fetchone()[0]
        return {
            "videos": videos,
            "videos_ok": ok,
            "videos_failed": fail,
            "videos_pending": pending,
            "has_detail": has_detail,
            "has_comments": has_comments,
            "has_danmaku": has_danmaku,
            "comments": self._conn.execute("SELECT COUNT(*) FROM collected_comments").fetchone()[0],
            "danmaku": self._conn.execute("SELECT COUNT(*) FROM collected_danmaku").fetchone()[0],
            "ups": self._conn.execute("SELECT COUNT(*) FROM collected_ups").fetchone()[0],
            "up_failed": self._conn.execute("SELECT COUNT(*) FROM failed_ups").fetchone()[0],
        }

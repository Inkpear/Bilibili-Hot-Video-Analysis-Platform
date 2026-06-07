# etl/db.py
import pymysql
import os
import logging

logger = logging.getLogger(__name__)


class Database:
    def __init__(self):
        self.conn = pymysql.connect(
            host=os.getenv('DB_HOST', 'localhost'),
            user=os.getenv('DB_USER', 'root'),
            password=os.getenv('DB_PASSWORD', ''),
            database=os.getenv('DB_NAME', 'bilibili'),
            charset='utf8mb4',
            cursorclass=pymysql.cursors.DictCursor,
        )

    def insert_video(self, video: dict, snapshot_date: str):
        """Insert or update video data."""
        sql = """
        INSERT INTO hot_video
        (bvid, snapshot_date, aid, cid, title, description, tags, sub_category,
         duration, publish_time, up_id, up_name, up_avatar,
         view_count, like_count, coin_count, favorite_count,
         share_count, reply_count, danmaku_count)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        view_count=VALUES(view_count), like_count=VALUES(like_count),
        coin_count=VALUES(coin_count), favorite_count=VALUES(favorite_count),
        share_count=VALUES(share_count), reply_count=VALUES(reply_count),
        danmaku_count=VALUES(danmaku_count)
        """
        try:
            with self.conn.cursor() as cursor:
                cursor.execute(sql, (
                    video['bvid'], snapshot_date,
                    video.get('aid'), video.get('cid'),
                    video.get('title', ''), video.get('description', ''),
                    video.get('tags', ''), video.get('sub_category', ''),
                    video.get('duration', 0), video.get('publish_time', ''),
                    video.get('up_id', 0), video.get('up_name', ''),
                    video.get('up_avatar', ''),
                    video.get('view_count', 0), video.get('like_count', 0),
                    video.get('coin_count', 0), video.get('favorite_count', 0),
                    video.get('share_count', 0), video.get('reply_count', 0),
                    video.get('danmaku_count', 0),
                ))
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert video %s: %s", video.get('bvid'), e)

    def insert_interaction(self, bvid: str, snapshot_date: str, rates: dict):
        """Insert or update interaction metrics."""
        sql = """
        INSERT INTO video_interaction
        (bvid, snapshot_date, like_rate, coin_rate, fav_rate, reply_rate, share_rate)
        VALUES (%s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        like_rate=VALUES(like_rate), coin_rate=VALUES(coin_rate),
        fav_rate=VALUES(fav_rate), reply_rate=VALUES(reply_rate),
        share_rate=VALUES(share_rate)
        """
        try:
            with self.conn.cursor() as cursor:
                cursor.execute(sql, (
                    bvid, snapshot_date,
                    rates.get('like_rate', 0), rates.get('coin_rate', 0),
                    rates.get('fav_rate', 0), rates.get('reply_rate', 0),
                    rates.get('share_rate', 0),
                ))
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert interaction for %s: %s", bvid, e)

    def insert_word_cloud(self, table: str, bvid: str, word: str,
                          snapshot_date: str, count: int, category: str):
        """Insert or update word cloud data into a specified table."""
        sql = f"""
        INSERT INTO {table}
        (bvid, word, snapshot_date, word_count, category)
        VALUES (%s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE word_count=VALUES(word_count)
        """
        try:
            with self.conn.cursor() as cursor:
                cursor.execute(sql, (bvid, word, snapshot_date, count, category))
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert word cloud into %s: %s", table, e)

    def close(self):
        self.conn.close()

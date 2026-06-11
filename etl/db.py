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
            autocommit=False,
        )

    def commit(self):
        self.conn.commit()

    def rollback(self):
        self.conn.rollback()

    def insert_videos_batch(self, videos: list, snapshot_date: str):
        sql = """
        INSERT INTO hot_video
        (bvid, snapshot_date, aid, cid, title, description, tags, category, sub_category,
         duration, publish_time, up_id, up_name, up_avatar,
         view_count, like_count, coin_count, favorite_count,
         share_count, reply_count, danmaku_count,
         rank_name, rank_position, rank_source, url, snapshot_id, collect_time)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        category=VALUES(category), view_count=VALUES(view_count), like_count=VALUES(like_count),
        coin_count=VALUES(coin_count), favorite_count=VALUES(favorite_count),
        share_count=VALUES(share_count), reply_count=VALUES(reply_count),
        danmaku_count=VALUES(danmaku_count),
        rank_position=VALUES(rank_position), rank_source=VALUES(rank_source),
        rank_name=VALUES(rank_name), collect_time=VALUES(collect_time)
        """
        data = [(
            v['bvid'], snapshot_date, v.get('aid'), v.get('cid'),
            v.get('title', ''), v.get('description', ''), v.get('tags', ''),
            v.get('category', ''), v.get('sub_category', ''),
            v.get('duration', 0), v.get('publish_time', ''), v.get('up_id', 0),
            v.get('up_name', ''), v.get('up_avatar', ''),
            v.get('view_count', 0), v.get('like_count', 0), v.get('coin_count', 0),
            v.get('favorite_count', 0), v.get('share_count', 0), v.get('reply_count', 0),
            v.get('danmaku_count', 0), v.get('rank_name', ''), v.get('rank_position', 0),
            v.get('rank_source', ''), v.get('url', ''), v.get('snapshot_id', ''),
            v.get('collect_time', None),
        ) for v in videos]
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, data)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert videos batch: %s", e)

    def insert_interactions_batch(self, interactions: list):
        sql = """
        INSERT INTO video_interaction (bvid, snapshot_date, like_rate, coin_rate, fav_rate, reply_rate, share_rate)
        VALUES (%s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        like_rate=VALUES(like_rate), coin_rate=VALUES(coin_rate),
        fav_rate=VALUES(fav_rate), reply_rate=VALUES(reply_rate), share_rate=VALUES(share_rate)
        """
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, interactions)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert interactions batch: %s", e)

    def insert_up_info_batch(self, up_list: list, snapshot_date: str):
        sql = """
        INSERT INTO up_info (up_id, snapshot_date, up_name, up_avatar, follower_count, video_count, sign, official_verify)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        up_name=VALUES(up_name), up_avatar=VALUES(up_avatar),
        follower_count=VALUES(follower_count), video_count=VALUES(video_count),
        sign=VALUES(sign), official_verify=VALUES(official_verify)
        """
        data = [(
            u.get('up_id', ''), snapshot_date, u.get('up_name', ''), u.get('up_avatar', ''),
            u.get('follower_count', 0), u.get('video_count', 0),
            u.get('sign', ''), u.get('official_verify', ''),
        ) for u in up_list]
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, data)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert up_info batch: %s", e)

    def insert_comments_batch(self, comments: list, snapshot_date: str):
        if not comments:
            return
        sql = """
        INSERT INTO comment_info
        (comment_id, snapshot_date, bvid, aid, user_id, user_name, content, like_count, reply_count, comment_time)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE like_count=VALUES(like_count), reply_count=VALUES(reply_count)
        """
        data = [(
            c.get('comment_id'), snapshot_date, c.get('bvid'), c.get('aid'),
            c.get('user_id'), c.get('user_name'), c.get('content'),
            c.get('like_count', 0), c.get('reply_count', 0), c.get('comment_time'),
        ) for c in comments]
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, data)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert comments batch: %s", e)

    def insert_danmaku_batch(self, danmaku_list: list, snapshot_date: str):
        if not danmaku_list:
            return
        sql = """
        INSERT INTO danmaku_info
        (danmaku_id, snapshot_date, bvid, cid, content, video_time, mode, fontsize, color, mid_hash, send_time)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE content=VALUES(content)
        """
        data = [(
            d.get('danmaku_id'), snapshot_date, d.get('bvid'), d.get('cid'),
            d.get('content'), d.get('video_time'), d.get('mode'),
            d.get('fontsize'), d.get('color'), d.get('mid_hash'), d.get('send_time'),
        ) for d in danmaku_list]
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, data)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert danmaku batch: %s", e)

    def insert_word_cloud_batch(self, table: str, words: list, snapshot_date: str):
        if not words:
            return
        sql = f"""
        INSERT INTO {table} (bvid, word, snapshot_date, word_count, category)
        VALUES (%s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE word_count=VALUES(word_count)
        """
        data = [(bvid, word, snapshot_date, count, cat) for bvid, word, count, cat in words]
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, data)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert word cloud batch into %s: %s", table, e)

    def insert_word_trend_batch(self, trends: list, snapshot_date: str):
        if not trends:
            return
        sql = """
        INSERT INTO word_trend (word, snapshot_date, category, source_type, total_count)
        VALUES (%s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE total_count=VALUES(total_count)
        """
        data = [(word, snapshot_date, category, source_type, count) for word, category, source_type, count in trends]
        try:
            with self.conn.cursor() as cursor:
                cursor.executemany(sql, data)
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert word trend batch: %s", e)

    def insert_snapshot_date(self, snapshot_date: str, video_count: int):
        sql = """
        INSERT INTO snapshot_dates (snapshot_date, video_count)
        VALUES (%s, %s)
        ON DUPLICATE KEY UPDATE video_count=VALUES(video_count)
        """
        try:
            with self.conn.cursor() as cursor:
                cursor.execute(sql, (snapshot_date, video_count))
            self.conn.commit()
        except pymysql.Error as e:
            self.conn.rollback()
            logger.error("Failed to insert snapshot date: %s", e)

    def close(self):
        self.conn.close()

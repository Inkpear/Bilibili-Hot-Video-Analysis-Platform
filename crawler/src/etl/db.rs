use sqlx::mysql::{MySqlPool, MySql};
use sqlx::QueryBuilder;
use tracing::info;

use crate::error::Result;
use crate::models::{Comment, Danmaku, UpInfo, VideoDetail};

const BATCH_SIZE: usize = 5000;

pub struct MysqlDb {
    pool: MySqlPool,
}

impl MysqlDb {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = MySqlPool::connect(url).await?;
        info!("MySQL connected");
        Ok(Self { pool })
    }

    /// 开启 ETL 批量写入事务，返回事务对象
    /// 调用方用完后需调用 `commit_etl_tx` 提交
    pub async fn begin_etl_tx(&self) -> Result<sqlx::Transaction<'static, MySql>> {
        let tx = self.pool.begin().await?;
        Ok(tx)
    }

    /// 获取底层连接池（供 insert 方法在事务外使用）
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    pub async fn insert_videos_batch(&self, videos: &[VideoDetail], date: &str) -> Result<()> {
        for chunk in videos.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO hot_video (bvid, snapshot_date, aid, cid, title, description, tags, \
                 sub_category, category, duration, publish_time, up_id, up_name, up_avatar, \
                 view_count, like_count, coin_count, favorite_count, share_count, \
                 reply_count, danmaku_count, rank_name, rank_position, rank_source, \
                 url, snapshot_id, collect_time) "
            );
            qb.push_values(chunk, |mut b, v| {
                b.push_bind(&v.bvid)
                    .push_bind(date)
                    .push_bind(v.aid)
                    .push_bind(v.cid)
                    .push_bind(&v.title)
                    .push_bind(&v.description)
                    .push_bind(&v.tags)
                    .push_bind(&v.sub_category)
                    .push_bind(&v.category)
                    .push_bind(v.duration)
                    .push_bind(v.publish_time)
                    .push_bind(&v.up_id)
                    .push_bind(&v.up_name)
                    .push_bind(&v.up_avatar)
                    .push_bind(v.view_count)
                    .push_bind(v.like_count)
                    .push_bind(v.coin_count)
                    .push_bind(v.favorite_count)
                    .push_bind(v.share_count)
                    .push_bind(v.reply_count)
                    .push_bind(v.danmaku_count)
                    .push_bind(&v.rank_name)
                    .push_bind(v.rank_position)
                    .push_bind(&v.rank_source)
                    .push_bind(&v.url)
                    .push_bind(&v.snapshot_id)
                    .push_bind(v.collect_time);
            });
            qb.push(" ON DUPLICATE KEY UPDATE category=VALUES(category), view_count=VALUES(view_count), like_count=VALUES(like_count), coin_count=VALUES(coin_count), favorite_count=VALUES(favorite_count), share_count=VALUES(share_count), reply_count=VALUES(reply_count), danmaku_count=VALUES(danmaku_count), rank_position=VALUES(rank_position), rank_source=VALUES(rank_source), rank_name=VALUES(rank_name), collect_time=VALUES(collect_time)");
            qb.build().execute(&self.pool).await?;
        }
        info!(count = videos.len(), "inserted hot_video");
        Ok(())
    }

    pub async fn insert_interactions_batch(&self, videos: &[VideoDetail], date: &str) -> Result<()> {
        for chunk in videos.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO video_interaction (bvid, snapshot_date, like_rate, coin_rate, fav_rate, reply_rate, share_rate) "
            );
            qb.push_values(chunk, |mut b, v| {
                let view = v.view_count.max(1) as f64;
                b.push_bind(&v.bvid)
                    .push_bind(date)
                    .push_bind(v.like_count as f64 / view)
                    .push_bind(v.coin_count as f64 / view)
                    .push_bind(v.favorite_count as f64 / view)
                    .push_bind(v.reply_count as f64 / view)
                    .push_bind(v.share_count as f64 / view);
            });
            qb.push(" ON DUPLICATE KEY UPDATE like_rate=VALUES(like_rate), coin_rate=VALUES(coin_rate), fav_rate=VALUES(fav_rate), reply_rate=VALUES(reply_rate), share_rate=VALUES(share_rate)");
            qb.build().execute(&self.pool).await?;
        }
        info!(count = videos.len(), "inserted video_interaction");
        Ok(())
    }

    pub async fn insert_up_info_batch(&self, ups: &[UpInfo], date: &str) -> Result<()> {
        for chunk in ups.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO up_info (up_id, snapshot_date, up_name, up_avatar, follower_count, video_count, sign, official_verify) "
            );
            qb.push_values(chunk, |mut b, u| {
                b.push_bind(&u.up_id)
                    .push_bind(date)
                    .push_bind(&u.up_name)
                    .push_bind(&u.up_avatar)
                    .push_bind(u.follower_count)
                    .push_bind(u.video_count)
                    .push_bind(&u.sign)
                    .push_bind(&u.official_verify);
            });
            qb.push(" ON DUPLICATE KEY UPDATE up_name=VALUES(up_name), up_avatar=VALUES(up_avatar), follower_count=VALUES(follower_count), video_count=VALUES(video_count), sign=VALUES(sign), official_verify=VALUES(official_verify)");
            qb.build().execute(&self.pool).await?;
        }
        info!(count = ups.len(), "inserted up_info");
        Ok(())
    }

    pub async fn insert_comments_batch(&self, comments: &[Comment], date: &str) -> Result<()> {
        for chunk in comments.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO comment_info (comment_id, snapshot_date, bvid, aid, user_id, user_name, content, like_count, reply_count, comment_time) "
            );
            qb.push_values(chunk, |mut b, c| {
                b.push_bind(c.comment_id)
                    .push_bind(date)
                    .push_bind(&c.bvid)
                    .push_bind(c.aid)
                    .push_bind(c.user_id)
                    .push_bind(&c.user_name)
                    .push_bind(&c.content)
                    .push_bind(c.like_count)
                    .push_bind(c.reply_count)
                    .push_bind(c.comment_time);
            });
            qb.push(" ON DUPLICATE KEY UPDATE like_count=VALUES(like_count), reply_count=VALUES(reply_count)");
            qb.build().execute(&self.pool).await?;
        }
        info!(count = comments.len(), "inserted comment_info");
        Ok(())
    }

    pub async fn insert_danmaku_batch(&self, danmaku_list: &[Danmaku], date: &str) -> Result<()> {
        for chunk in danmaku_list.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO danmaku_info (danmaku_id, snapshot_date, bvid, cid, content, video_time, mode, fontsize, color, mid_hash, send_time) "
            );
            qb.push_values(chunk, |mut b, d| {
                b.push_bind(d.danmaku_id)
                    .push_bind(date)
                    .push_bind(&d.bvid)
                    .push_bind(d.cid)
                    .push_bind(&d.content)
                    .push_bind(d.video_time)
                    .push_bind(d.mode)
                    .push_bind(d.fontsize)
                    .push_bind(d.color)
                    .push_bind(&d.mid_hash)
                    .push_bind(d.send_time);
            });
            qb.push(" ON DUPLICATE KEY UPDATE content=VALUES(content)");
            qb.build().execute(&self.pool).await?;
        }
        info!(count = danmaku_list.len(), "inserted danmaku_info");
        Ok(())
    }

    pub async fn insert_word_cloud(
        &self,
        table: &str,
        entries: &[(String, String, String, i64)],
        date: &str,
    ) -> Result<()> {
        let (sql_prefix, on_dup) = match table {
            "word_cloud_title" => (
                "INSERT INTO word_cloud_title (bvid, word, snapshot_date, word_count, sub_category) ",
                " ON DUPLICATE KEY UPDATE word_count=VALUES(word_count)",
            ),
            "word_cloud_comment" => (
                "INSERT INTO word_cloud_comment (bvid, word, snapshot_date, word_count, sub_category) ",
                " ON DUPLICATE KEY UPDATE word_count=VALUES(word_count)",
            ),
            "word_cloud_danmaku" => (
                "INSERT INTO word_cloud_danmaku (bvid, word, snapshot_date, word_count, sub_category) ",
                " ON DUPLICATE KEY UPDATE word_count=VALUES(word_count)",
            ),
            _ => return Err(crate::error::CrawlerError::Config(format!("unknown word cloud table: {}", table))),
        };

        for chunk in entries.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(sql_prefix);
            qb.push_values(chunk, |mut b, (bvid, word, category, count)| {
                b.push_bind(bvid)
                    .push_bind(word)
                    .push_bind(date)
                    .push_bind(count)
                    .push_bind(category);
            });
            qb.push(on_dup);
            qb.build().execute(&self.pool).await?;
        }
        info!(table = table, count = entries.len(), "inserted word_cloud");
        Ok(())
    }

    pub async fn insert_word_trend(
        &self,
        entries: &[(String, String, String, i64)],
        date: &str,
    ) -> Result<()> {
        for chunk in entries.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO word_trend (word, snapshot_date, category, source_type, total_count) "
            );
            qb.push_values(chunk, |mut b, (word, category, source_type, total_count)| {
                b.push_bind(word)
                    .push_bind(date)
                    .push_bind(category)
                    .push_bind(source_type)
                    .push_bind(total_count);
            });
            qb.push(" ON DUPLICATE KEY UPDATE total_count=VALUES(total_count)");
            qb.build().execute(&self.pool).await?;
        }
        info!(count = entries.len(), "inserted word_trend");
        Ok(())
    }

    pub async fn insert_snapshot_date(
        &self,
        date: &str,
        video_count: usize,
        comment_count: usize,
        danmaku_count: usize,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO snapshot_dates (snapshot_date, video_count, comment_count, danmaku_count) \
             VALUES (?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE video_count=VALUES(video_count), comment_count=VALUES(comment_count), danmaku_count=VALUES(danmaku_count)",
        )
        .bind(date)
        .bind(video_count as i64)
        .bind(comment_count as i64)
        .bind(danmaku_count as i64)
        .execute(&self.pool)
        .await?;
        info!(date = date, "inserted snapshot_dates");
        Ok(())
    }
}

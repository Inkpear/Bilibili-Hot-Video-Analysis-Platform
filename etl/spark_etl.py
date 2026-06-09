from pyspark.sql import SparkSession
from pyspark.sql.functions import *
from pyspark.sql.types import *
import jieba
import re
import argparse
import subprocess

STOPWORDS = {
    '的', '了', '在', '是', '我', '有', '和', '就', '不', '人',
    '都', '一', '一个', '上', '也', '很', '到', '说', '要', '去',
    '你', '会', '着', '没有', '看', '好', '自己', '这', '他', '她',
    '哈哈哈', '666', '来了', '好的', '可以', '什么', '怎么',
    '视频', '播放', '投稿', '热门', '合集', '完整版', '全程',
}

def segment_text(text):
    if not text:
        return []
    text = re.sub(r'[^\w\s]', '', str(text))
    words = jieba.lcut(text)
    return [w for w in words if w not in STOPWORDS and len(w) > 1]

segment_udf = udf(segment_text, ArrayType(StringType()))

def generate_word_cloud(df, text_col, bvid_col, date_col, category_col, table_name, mysql_url, mysql_props):
    words = df.select(
        col(bvid_col),
        col(category_col).alias("sub_category"),
        explode(segment_udf(text_col)).alias("word"),
        col(date_col)
    )
    word_counts = words.groupBy(bvid_col, "word", date_col, "sub_category") \
        .agg(count("*").alias("word_count"))
    word_counts.write.jdbc(url=mysql_url, table=table_name, mode="overwrite", properties=mysql_props)
    return word_counts

def get_hdfs_dates():
    try:
        result = subprocess.run(
            ["hdfs", "dfs", "-ls", "/bilibili/raw/video"],
            capture_output=True, text=True, timeout=10
        )
        dates = []
        for line in result.stdout.split('\n'):
            if 'date=' in line:
                dates.append(line.split('date=')[-1].strip())
        return sorted(set(dates))
    except:
        return []

def main():
    parser = argparse.ArgumentParser(description='Bilibili Unified ETL Pipeline')
    parser.add_argument('--dates', help='Comma-separated dates or range YYYYMMDD-YYYYMMDD')
    args = parser.parse_args()

    spark = SparkSession.builder \
        .appName("Bilibili ETL") \
        .config("spark.sql.warehouse.dir", "/user/hive/warehouse") \
        .getOrCreate()

    mysql_url = "jdbc:mysql://mysql:3306/bilibili"
    mysql_props = {
        "user": "root",
        "password": "root",
        "driver": "com.mysql.cj.jdbc.Driver"
    }

    if args.dates:
        if '-' in args.dates:
            start, end = args.dates.split('-')
            from datetime import datetime, timedelta
            d = datetime.strptime(start, '%Y%m%d')
            end_d = datetime.strptime(end, '%Y%m%d')
            dates = []
            while d <= end_d:
                dates.append(d.strftime('%Y%m%d'))
                d += timedelta(days=1)
        else:
            dates = args.dates.split(',')
    else:
        dates = get_hdfs_dates()
        if not dates:
            print("No dates found in HDFS. Use --dates to specify.")
            return

    print(f"Processing dates: {dates}")

    all_hot_videos = []
    all_interactions = []
    all_comment_infos = []
    all_danmaku_infos = []
    all_snapshot_dates = []
    all_up_infos = []

    for date in dates:
        print(f"Processing date: {date}")

        video_path = f"hdfs://master:9000/bilibili/raw/video/date={date}/*.ndjson"
        video_df = spark.read.option("mode", "PERMISSIVE") \
            .option("columnNameOfCorruptRecord", "_corrupt_record") \
            .json(video_path)
        if "_corrupt_record" in video_df.columns:
            video_df = video_df.filter(col("_corrupt_record").isNull()).drop("_corrupt_record")

        video_clean = video_df.dropDuplicates(["bvid"]) \
            .filter(col("view_count") > 0) \
            .withColumn("like_rate", col("like_count") / col("view_count")) \
            .withColumn("coin_rate", col("coin_count") / col("view_count")) \
            .withColumn("fav_rate", col("favorite_count") / col("view_count")) \
            .withColumn("reply_rate", col("reply_count") / col("view_count")) \
            .withColumn("share_rate", col("share_count") / col("view_count"))

        hot_video_cols = ["bvid", "aid", "cid", "title", "description", "tags", "sub_category",
                          "category", "duration", "publish_time", "up_id", "up_name", "up_avatar",
                          "view_count", "like_count", "coin_count", "favorite_count",
                          "share_count", "reply_count", "danmaku_count",
                          "rank_name", "rank_position", "rank_source",
                          "url", "snapshot_id", "collect_time"]
        hot_video = video_clean.select(hot_video_cols).withColumn("snapshot_date", lit(date))
        all_hot_videos.append(hot_video)

        interaction = video_clean.select(
            col("bvid"), lit(date).alias("snapshot_date"),
            col("like_rate"), col("coin_rate"), col("fav_rate"),
            col("reply_rate"), col("share_rate")
        )
        all_interactions.append(interaction)

        comment_count = 0
        comment_path = f"hdfs://master:9000/bilibili/raw/comment/date={date}/*.ndjson"
        try:
            comment_df = spark.read.json(comment_path)
            comment_clean = comment_df.dropDuplicates(["comment_id"])
            comment_info = comment_clean.withColumn("snapshot_date", lit(date))
            all_comment_infos.append(comment_info)
            comment_count = comment_clean.count()
        except Exception as e:
            print(f"  Comment error: {e}")

        danmaku_count = 0
        danmaku_path = f"hdfs://master:9000/bilibili/raw/danmaku/date={date}/*.ndjson"
        try:
            danmaku_df = spark.read.json(danmaku_path)
            danmaku_clean = danmaku_df.dropDuplicates(["danmaku_id"])
            danmaku_info = danmaku_clean.withColumn("snapshot_date", lit(date))
            all_danmaku_infos.append(danmaku_info)
            danmaku_count = danmaku_clean.count()
        except Exception as e:
            print(f"  Danmaku error: {e}")

        up_info_path = f"hdfs://master:9000/bilibili/raw/up_info/date={date}/*.ndjson"
        try:
            up_info_df = spark.read.json(up_info_path)
            up_info_clean = up_info_df.dropDuplicates(["up_id"])
            up_info_with_date = up_info_clean.withColumn("snapshot_date", lit(date))
            all_up_infos.append(up_info_with_date)
        except Exception as e:
            print(f"  Up_info error: {e}")

        video_count = video_clean.count()
        snapshot_df = spark.createDataFrame(
            [(date, video_count, comment_count, danmaku_count)],
            ["snapshot_date", "video_count", "comment_count", "danmaku_count"]
        )
        all_snapshot_dates.append(snapshot_df)

        print(f"  Videos: {video_count}, Comments: {comment_count}, Danmaku: {danmaku_count}")

    print("Writing to MySQL...")

    def union_and_write(dfs, table_name):
        if not dfs:
            return
        combined = dfs[0]
        for df in dfs[1:]:
            combined = combined.unionByName(df, allowMissingColumns=True)
        combined.write.jdbc(url=mysql_url, table=table_name, mode="overwrite", properties=mysql_props)
        print(f"  {table_name}: {combined.count()} rows")

    union_and_write(all_hot_videos, "hot_video")
    union_and_write(all_interactions, "video_interaction")
    union_and_write(all_comment_infos, "comment_info")
    union_and_write(all_danmaku_infos, "danmaku_info")
    union_and_write(all_snapshot_dates, "snapshot_dates")
    union_and_write(all_up_infos, "up_info")

    print("Generating word clouds...")

    all_videos_df = all_hot_videos[0]
    for df in all_hot_videos[1:]:
        all_videos_df = all_videos_df.unionByName(df, allowMissingColumns=True)

    title_wordcloud = generate_word_cloud(
        all_videos_df, "title", "bvid", "snapshot_date", "sub_category",
        "word_cloud_title", mysql_url, mysql_props
    )
    print(f"  word_cloud_title: {title_wordcloud.count()} rows")

    if all_comment_infos:
        all_comments_df = all_comment_infos[0]
        for df in all_comment_infos[1:]:
            all_comments_df = all_comments_df.unionByName(df, allowMissingColumns=True)
        all_comments_df = all_comments_df.withColumn("sub_category", lit(""))
        comment_wordcloud = generate_word_cloud(
            all_comments_df, "content", "bvid", "snapshot_date", "sub_category",
            "word_cloud_comment", mysql_url, mysql_props
        )
        print(f"  word_cloud_comment: {comment_wordcloud.count()} rows")

        word_trend_comment = comment_wordcloud.groupBy("word", "snapshot_date") \
            .agg(sum("word_count").alias("total_count")) \
            .withColumn("source_type", lit("comment")) \
            .withColumn("category", lit(""))
        all_word_trends = [word_trend_comment]

    if all_danmaku_infos:
        all_danmaku_df = all_danmaku_infos[0]
        for df in all_danmaku_infos[1:]:
            all_danmaku_df = all_danmaku_df.unionByName(df, allowMissingColumns=True)
        all_danmaku_df = all_danmaku_df.withColumn("sub_category", lit(""))
        danmaku_wordcloud = generate_word_cloud(
            all_danmaku_df, "content", "bvid", "snapshot_date", "sub_category",
            "word_cloud_danmaku", mysql_url, mysql_props
        )
        print(f"  word_cloud_danmaku: {danmaku_wordcloud.count()} rows")

        word_trend_danmaku = danmaku_wordcloud.groupBy("word", "snapshot_date") \
            .agg(sum("word_count").alias("total_count")) \
            .withColumn("source_type", lit("danmaku")) \
            .withColumn("category", lit(""))
        all_word_trends.append(word_trend_danmaku)

    word_trend_title = title_wordcloud.groupBy("word", "snapshot_date") \
        .agg(sum("word_count").alias("total_count")) \
        .withColumn("source_type", lit("title")) \
        .withColumn("category", lit(""))
    all_word_trends.append(word_trend_title)

    word_trend_union = all_word_trends[0]
    for df in all_word_trends[1:]:
        word_trend_union = word_trend_union.unionByName(df, allowMissingColumns=True)
    word_trend_union.write.jdbc(url=mysql_url, table="word_trend", mode="overwrite", properties=mysql_props)
    print(f"  word_trend: {word_trend_union.count()} rows")

    spark.stop()
    print("ETL completed!")

if __name__ == "__main__":
    main()

from pyspark.sql import SparkSession
from pyspark.sql.functions import *
from pyspark.sql.types import *

def main():
    spark = SparkSession.builder \
        .appName("Bilibili ETL") \
        .enableHiveSupport() \
        .getOrCreate()
    
    date = spark.conf.get("spark.etl.date")
    
    # 1. 读取ODS层数据
    video_df = spark.table(f"bilibili.ods_video").where(col("date") == date)
    comment_df = spark.table(f"bilibili.ods_comment").where(col("date") == date)
    
    # 2. 数据清洗
    video_clean = video_df.dropDuplicates(["bvid"]) \
        .filter(col("view_count") > 0)
    
    # 3. 计算互动指标
    video_with_rates = video_clean \
        .withColumn("like_rate", col("like_count") / col("view_count")) \
        .withColumn("coin_rate", col("coin_count") / col("view_count")) \
        .withColumn("fav_rate", col("favorite_count") / col("view_count")) \
        .withColumn("reply_rate", col("reply_count") / col("view_count")) \
        .withColumn("share_rate", col("share_count") / col("view_count"))
    
    # 4. 写入DWD层
    video_with_rates.write \
        .mode("overwrite") \
        .partitionBy("date") \
        .saveAsTable("bilibili.dwd_video_clean")
    
    # 5. 计算分区统计
    partition_stats = video_clean.groupBy("sub_category") \
        .agg(
            count("*").alias("video_count"),
            sum("view_count").alias("total_views"),
            sum("like_count").alias("total_likes"),
            avg(col("like_count") / col("view_count")).alias("avg_like_rate")
        )
    
    partition_stats.write \
        .mode("overwrite") \
        .saveAsTable("bilibili.dws_partition_stats")
    
    # 6. 同步到ADS层（供MySQL查询）
    video_with_rates.write \
        .mode("overwrite") \
        .partitionBy("date") \
        .saveAsTable("bilibili.ads_hot_video")
    
    spark.stop()

if __name__ == "__main__":
    main()

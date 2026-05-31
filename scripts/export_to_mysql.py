#!/usr/bin/env python3
"""
export_to_mysql.py — Spark ETL 结果导出到 MySQL

运行方式 (在 master 节点):
  spark-submit --jars /opt/spark/jars/mysql-connector-j-8.0.33.jar \\
      --master spark://master:7077 \\
      export_to_mysql.py

前提:
  - MySQL 在 master:3306，数据库 bilibili 已建立
  - MySQL 用户 hive / Hive@123456 有 bilibili.* 权限
  - mysql-connector-j jar 在 /opt/spark/jars/ 下
"""

import sys

from pyspark.sql import SparkSession

MYSQL_URL = "jdbc:mysql://master:3306/bilibili"
MYSQL_USER = "hive"
MYSQL_PASS = "Hive@123456"
MYSQL_DRIVER = "com.mysql.cj.jdbc.Driver"

TABLES = [
    "ads_screen_overview",
    "ads_hot_video_rank",
    "ads_partition_compare",
    "ads_video_lifecycle",
    "ads_partition_daily",
    "ads_up_summary",
    "ads_tag_cloud",
]

spark = SparkSession.builder \
    .appName("Bilibili-ADS-Export") \
    .enableHiveSupport() \
    .getOrCreate()

failed = False

for tbl in TABLES:
    try:
        df = spark.sql(f"SELECT * FROM bilibili.{tbl}")
        count = df.count()

        df.write \
            .format("jdbc") \
            .option("url", MYSQL_URL) \
            .option("dbtable", tbl) \
            .option("user", MYSQL_USER) \
            .option("password", MYSQL_PASS) \
            .option("driver", MYSQL_DRIVER) \
            .option("truncate", "true") \
            .mode("overwrite") \
            .save()

        print(f"  {tbl}: {count} rows exported ✓")

    except Exception as e:
        print(f"  {tbl}: FAILED - {e}")
        failed = True

spark.stop()
if failed:
    print("Export failed.")
    sys.exit(1)
print("Export complete.")

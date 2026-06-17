# 数据管线修复记录

> 日期：2026-06-01

## 背景

首轮爬虫采集完成（20260531_1842），数据在 WSL `~/output/20260531_1842/`。需要验证数据管线全链路：采集 → HDFS → Hive/Spark ETL → MapReduce 词云 → MySQL 导出。

## 采集数据概况

| 文件 | 行数 | 大小 |
|---|---|---|
| `rank_video.ndjson` | 954 | 821 KB |
| `up_info.ndjson` | 940 | 277 KB |
| `comments/*.ndjson` | 952 文件 | 51 MB |
| `danmaku/*.ndjson` | 886 文件 | 71 MB |

## 发现并修复的问题

### P1: upload_hdfs.sh 上传极慢（~5 小时）

**现象**：弹幕上传 517/886 文件耗时 96 分钟，约 11 秒/文件。

**根因**：每个小文件（平均 54-82 KB）都建立一次独立 SSH 连接。TCP 握手 + SSH 密钥交换 + 认证 + HDFS 元数据操作 ≈ 2-4 秒开销，实际数据传输只需毫秒级。总连接数 952 + 886 = 1,838 次。

**修复**：comments 和 danmaku 改为 `cat dir/*.ndjson | ssh hdfs dfs -put -f -` 合并上传，SSH 连接从 1,838 次降为 4 次。

**文件**：`scripts/upload_hdfs.sh`

### P2: SSH 日志 post-quantum 警告刷屏

**现象**：每条 SSH 命令输出 4 行 `WARNING: connection is not using a post-quantum key exchange algorithm`。

**根因**：WSL 的 OpenSSH 10.2p1 连接 CentOS 7 的 OpenSSH 7.4p1 时，客户端输出警告。

**修复**：WSL `~/.ssh/config` 添加：
```
Host 192.168.100.*
    LogLevel ERROR
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
```

### P3: Spark classpath 缺少 JsonSerDe jar（阻塞 ETL）

**现象**：Spark SQL 查询 ODS 表报 `ClassNotFoundException: org.apache.hive.hcatalog.data.JsonSerDe`。

**根因**：ODS 外部表使用 `JsonSerDe`，该类在 `hive-hcatalog-core-3.1.3.jar` 中，但该 jar 不在 `/opt/spark/jars/`。

**修复**：在 master、worker1、worker2 三节点都创建符号链接：
```bash
ln -sf /opt/hive/lib/hive-hcatalog-core-3.1.3.jar /opt/spark/jars/
```

### P4: Spark executor ClassCastException（SerializedLambda）

**现象**：`java.lang.ClassCastException: cannot assign instance of java.lang.invoke.SerializedLambda to field org.apache.spark.rdd.MapPartitionsRDD.f`，错误发生在 worker 节点。

**根因**：`hive-hcatalog-core` jar 只在 master 上创建了符号链接，worker 节点没有。driver 和 executor classpath 不匹配导致序列化失败。

**修复**：在 worker1、worker2 上也创建符号链接（同 P3）。

### P5: ods_up_raw.up_id 类型不匹配

**现象**：JsonSerDe 解析时报 `JsonParseException: Current token (VALUE_STRING) not numeric`。

**根因**：`ods_up_raw` 表定义 `up_id BIGINT`，但爬虫输出的 JSON 中 `up_id` 是字符串（如 `"3546381809617159"`）。

**修复**：`sql/hive_ods_ddl.sql` 中 `up_id` 从 `BIGINT` 改为 `STRING`。

### P6: dwd_comment_info / dwd_danmaku_info TIMESTAMP 转换失败

**现象**：`Cannot safely cast "STRING" to "TIMESTAMP"`。

**根因**：`FROM_UNIXTIME()` 返回 STRING，而表定义为 TIMESTAMP。

**修复**：`sql/spark_etl.sql` 中加外层 CAST：
```sql
-- 修改前
FROM_UNIXTIME(CAST(comment_time AS BIGINT)) AS comment_time
-- 修改后
CAST(FROM_UNIXTIME(CAST(comment_time AS BIGINT)) AS TIMESTAMP) AS comment_time
```
`send_time` 同理。

### P7: dwd_up_info.up_id 类型不匹配

**现象**：与 P5 同源，`dwd_up_info` 表也定义了 `up_id BIGINT`。

**修复**：`sql/spark_etl.sql` 中 `dwd_up_info.up_id` 从 `BIGINT` 改为 `STRING`，同时简化 `dws_up_summary_stats` 的 JOIN 条件（去掉 `CAST(u.up_id AS STRING)`）。

### P8: mapred-site.xml 缺少 HADOOP_MAPRED_HOME 配置

**现象**：Hadoop Streaming 报错要求配置 `yarn.app.mapreduce.am.env` 等三项。

**修复**：master 节点 `/opt/hadoop/etc/hadoop/mapred-site.xml` 添加：
```xml
<property>
  <name>yarn.app.mapreduce.am.env</name>
  <value>HADOOP_MAPRED_HOME=/opt/hadoop</value>
</property>
<!-- mapreduce.map.env, mapreduce.reduce.env 同理 -->
```

### P9: run_tag_cloud_mr.sh 的 -partitioner 参数失败

**现象**：Hadoop Streaming 报 `Unrecognized option: -files`（实际是 `-partitioner` 导致整个命令解析失败）。

**根因**：脚本通过 SSH 远程执行时，`-partitioner org.apache.hadoop.mapred.lib.KeyFieldBasedPartitioner` 和相关 `-D` 参数在 shell 变量展开 + SSH 传递过程中解析异常。直接在 master 上执行相同命令正常。

**修复**：`scripts/run_tag_cloud_mr.sh` 中去掉 `-partitioner` 和 `-D mapreduce.partition.keypartitioner.options`，直接使用 jar 绝对路径。

### P10: Hive INSERT INTO 大数据量静默失败（已解决）

**现象**：
- `INSERT INTO TABLE ads_tag_cloud SELECT ... FROM mr_tag_cloud_raw LIMIT 10` → 成功写入 10 行
- `INSERT INTO TABLE ads_tag_cloud SELECT ... FROM mr_tag_cloud_raw`（无 LIMIT，~500 行）→ 写入 0 行，无报错
- `CREATE TABLE AS SELECT ...` 同样 0 行

**影响**：`ads_tag_cloud` 词云表无法通过 Spark SQL 导入数据。其他 6 张 ADS 表（overview、hot_video_rank、partition_compare、lifecycle、partition_daily、up_summary）均正常。

**根因分析**：
1. **MR 输出问题**：MapReduce 输出所有 `tag_count` 都是 1，`tag_count >= 2` 过滤掉了所有行
2. **Hadoop Streaming 配置问题**：原脚本使用 `-files`（复数）作为 Streaming 选项，但 Hadoop Streaming 不支持此选项
3. **文件分发失败**：`-files` 参数导致 reducer.py 没有被分发到 worker 节点，reducer 执行失败
4. **输出为 mapper 直接输出**：reducer 未执行，输出只是 mapper 的直接输出，所有计数都是 1

**修复方案**：
1. **修复 Hadoop Streaming 参数**：将 `-files mapper.py,reducer.py,stopwords.txt` 改为 `-file mapper.py -file reducer.py -file stopwords.txt`
2. **修复脚本架构**：修改 `run_tag_cloud_mr.sh` 支持本地和远程两种运行模式，避免嵌套 SSH 问题
3. **修复字符集问题**：修改 `export_to_mysql.py` 的 JDBC URL 添加 `characterEncoding=utf8` 参数
4. **修复编码问题**：将 `print` 语句中的 `✓` 字符改为 `OK`，避免 ASCII 编码错误

**验证结果**：
- MR 输出 934 行，其中 42 行 `tag_count >= 2`
- Spark SQL 写入 `ads_tag_cloud` 表成功，42 行数据
- MySQL 导出成功，category 字段正确显示为中文（全站、游戏、科技、舞蹈、音乐）

## 验证结果

| 步骤 | 状态 | 说明 |
|---|---|---|
| upload_hdfs.sh | ✅ | 合并上传 ~30 秒完成 |
| Spark ETL (DWD/DWS/ADS) | ✅ | 6 张表数据正常 |
| MapReduce 词云 | ✅ | Hadoop Streaming 执行成功，934 行输出，42 行 count >= 2 |
| Hive 词云导入 | ✅ | Spark SQL 写入 ads_tag_cloud 成功，42 行数据 |
| MySQL 导出 | ✅ | 7 张表全部导出成功，category 字段正确显示中文 |

## 本地文件变更

| 文件 | 变更 |
|---|---|
| `scripts/upload_hdfs.sh` | comments/danmaku 改为合并上传 |
| `scripts/run_tag_cloud_mr.sh` | 去掉 `-partitioner`，改用 `-file`（单数）参数，支持本地/远程运行模式，使用 SQL 文件执行 |
| `scripts/export_to_mysql.py` | JDBC URL 添加 `characterEncoding=utf8`，`print` 语句改为 ASCII 字符 |
| `scripts/run_pipeline.sh` | 步骤 3 改为在 master 上直接执行 `run_tag_cloud_mr.sh` |
| `sql/hive_ods_ddl.sql` | `ods_up_raw.up_id` BIGINT → STRING |
| `sql/spark_etl.sql` | `comment_time`/`send_time` 加 CAST TIMESTAMP；`dwd_up_info.up_id` BIGINT → STRING |

## 集群配置变更（未入项目文件）

| 节点 | 文件 | 变更 |
|---|---|---|
| master/worker1/worker2 | `/opt/spark/jars/hive-hcatalog-core-3.1.3.jar` | 新增符号链接 → `/opt/hive/lib/` |
| master | `/opt/hadoop/etc/hadoop/mapred-site.xml` | 添加 HADOOP_MAPRED_HOME 三项配置 |
| master | `/opt/spark/conf/spark-defaults.conf` | 添加字符集配置 `spark.driver.extraJavaOptions=-Dfile.encoding=UTF-8` |
| WSL | `~/.ssh/config` | 添加 `LogLevel ERROR` 抑制警告 |

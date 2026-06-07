# B站热门视频分析平台 v2 — 部署配置与现状报告

> 日期：2026-06-07
> 作者：Sisyphus
> 状态：集群基础架构已搭建，Hive 部署卡在 Schema 初始化

---

## 一、项目目标

构建一个B站热门视频分析大屏，包含：
- Rust 爬虫（全量+增量采集）
- 大数据 ETL（HDFS → Spark/Hive → MySQL）
- Rust 后端 API（Axum + sqlx）
- React 前端大屏（ECharts + Tailwind CSS）

---

## 二、当前部署架构

### 2.1 服务器

| 角色 | 地址 | 说明 |
|------|------|------|
| WSL (开发机) | 通过 `ssh UbuntuDesktop` 访问 | Docker 宿主机 |
| 公网服务器 | 111.228.34.34 | 旧项目的前端/后端部署（已清理） |

### 2.2 Docker 集群拓扑

采用 Docker Compose 部署，逻辑上分为 3 节点：

**Node 1 — Master 节点：**
| 容器 | 镜像 | 端口 | 状态 |
|------|------|------|------|
| namenode | bde2020/hadoop-namenode:2.0.0-hadoop3.2.1-java8 | 9870, 9000 | ✅ 运行 |
| resourcemanager | bde2020/hadoop-resourcemanager:2.0.0-hadoop3.2.1-java8 | 8088 | ✅ 运行 |
| spark-master | bde2020/spark-master:3.0.0-hadoop3.2 | 8080, 7077 | ✅ 运行 |
| hive-metastore | bde2020/hive:2.3.2-postgresql-metastore | 9083 | ❌ Schema未初始化 |
| hive-server | bde2020/hive:2.3.2-postgresql-metastore | 10000 | ❌ 依赖metastore |
| hive-metastore-postgresql | postgres:13 | 5432 | ✅ 运行 |

**Node 2 — Worker 1：**
| 容器 | 镜像 | 状态 |
|------|------|------|
| datanode1 | bde2020/hadoop-datanode:2.0.0-hadoop3.2.1-java8 | ✅ 运行 |
| nodemanager1 | bde2020/hadoop-nodemanager:2.0.0-hadoop3.2.1-java8 | ✅ 运行 |
| spark-worker1 | bde2020/spark-worker:3.0.0-hadoop3.2 | ✅ 运行 |

**Node 3 — Worker 2：**
| 容器 | 镜像 | 状态 |
|------|------|------|
| datanode2 | bde2020/hadoop-datanode:2.0.0-hadoop3.2.1-java8 | ✅ 运行 |
| nodemanager2 | bde2020/hadoop-nodemanager:2.0.0-hadoop3.2.1-java8 | ✅ 运行 |
| spark-worker2 | bde2020/spark-worker:3.0.0-hadoop3.2 | ✅ 运行 |

**一次性初始化容器：**
| 容器 | 用途 | 状态 |
|------|------|------|
| hive-schema-init | 初始化 PostgreSQL 中的 Hive Metastore Schema | ❌ 失败 |
| hdfs-init | 创建 /tmp 和 /user/hive/warehouse 目录 | ✅ 成功 |

---

## 三、已验证通过的组件

| 组件 | 验证方式 | 结果 |
|------|----------|------|
| HDFS | `hdfs dfsadmin -report` → Live datanodes: 2 | ✅ |
| YARN | `yarn node -list` → Total Nodes: 2 | ✅ |
| Spark | Spark Master UI → Workers: 2 | ✅ |
| HDFS 目录 | `hdfs dfs -ls /` → /tmp, /user/hive/warehouse 已创建 | ✅ |
| PostgreSQL | 容器运行，端口 5432 可达 | ✅ |

---

## 四、Hive 部署问题详述

### 4.1 问题现象

```
MetaException(message:Version information not found in metastore.)
```

Hive Metastore 启动时检查 PostgreSQL 中的 schema_version 表，发现为空，直接崩溃。

### 4.2 根本原因

BDE 镜像的 `schematool` 命令存在 bug：
- 命令行传入 `-url jdbc:postgresql://...` 参数被忽略
- 始终使用 `jdbc:derby:;databaseName=metastore_db;create=true`
- Derby 不支持 `SET statement_timeout = 0` 语法，导致 schema 脚本执行失败

**证据：**
```bash
docker run --rm bde2020/hive:2.3.2-postgresql-metastore \
  /opt/hive/bin/schematool -dbType postgres -initSchema \
  -userName hive -passWord hive \
  -url jdbc:postgresql://hive-metastore-postgresql:5432/metastore

# 输出显示：
# Metastore connection URL: jdbc:derby:;databaseName=metastore_db;create=true
# ← 仍然是 Derby，不是 PostgreSQL
```

### 4.3 已尝试的方案

| 方案 | 结果 | 失败原因 |
|------|------|----------|
| BDE 镜像 + HIVE_SITE_CONF 环境变量 | ❌ | entrypoint 脚本覆盖了 schematool 的连接参数 |
| Apache 官方镜像 (apache/hive:3.1.3) | ❌ | Schema 初始化成功，但 HiveServer2 绑定 127.0.0.1:端口，Docker 端口映射无效 |
| 纯 postgres:13 + BDE hive + 显式 command | ❌ | schematool 仍使用 Derby |
| -dbType postgres -initSchema -url ... | ❌ | BDE 镜像的 schematool 忽略 -url 参数 |

### 4.4 Apache 官方镜像的补充信息

Apache 镜像 (apache/hive:3.1.3) 的 schema 初始化是成功的：
```
Initialization script completed
schemaTool completed
Initialized schema successfully..
```

但 HiveServer2 进程启动后绑定到 `127.0.0.1:36211`（从 /proc/net/tcp 看到），而非 `0.0.0.0:10000`，导致外部无法访问。

---

## 五、当前 docker-compose.yml 配置

文件位置：`~/hadoop-cluster/docker-compose.yml`（WSL）

```yaml
version: "3.7"

x-hadoop-env: &hadoop-env
  CORE_CONF_fs_defaultFS: "hdfs://namenode:9000"
  CORE_CONF_hadoop_http_staticuser_user: "root"
  HDFS_CONF_dfs_webhdfs_enabled: "true"
  YARN_CONF_yarn_resourcemanager_hostname: "resourcemanager"
  YARN_CONF_yarn_nodemanager_resource_memory__mb: "2048"

networks:
  bigdata_net:
    driver: bridge

services:
  # Hadoop
  namenode: ...
  resourcemanager: ...
  datanode1: ...
  datanode2: ...
  nodemanager1: ...
  nodemanager2: ...

  # Spark
  spark-master: ...
  spark-worker1: ...
  spark-worker2: ...

  # Hive
  hive-metastore-postgresql:
    image: postgres:13
    environment:
      POSTGRES_DB: metastore
      POSTGRES_USER: hive
      POSTGRES_PASSWORD: hive

  hive-schema-init:
    image: bde2020/hive:2.3.2-postgresql-metastore
    command: ["/bin/bash", "-c", "sleep 10 && schematool -dbType postgres -initSchema ..."]
    # ← 此处 schematool 仍使用 Derby，未连接 PostgreSQL

  hdfs-init:
    image: bde2020/hadoop-base:2.0.0-hadoop3.2.1-java8
    command: ["/bin/bash", "-c", "sleep 20 && hdfs dfs -mkdir -p /tmp ..."]
    # ← 此容器已成功执行

  hive-metastore:
    image: bde2020/hive:2.3.2-postgresql-metastore
    command: /opt/hive/bin/hive --service metastore
    depends_on: [hive-schema-init, namenode]

  hive-server:
    image: bde2020/hive:2.3.2-postgresql-metastore
    depends_on: [hive-metastore, hdfs-init]
```

---

## 六、建议的后续方案

### 方案 A：手动执行 SQL 初始化（推荐）

1. 进入 PostgreSQL 容器，手动创建 Hive Metastore schema
2. 绕过 schematool，直接执行 DDL

```bash
# 1. 下载 Hive 3.1.3 的 PostgreSQL schema 脚本
# 2. 进入 postgres 容器执行
docker exec -i hive-metastore-postgresql psql -U hive -d metastore < hive-schema-3.1.0.postgres.sql
```

### 方案 B：使用 Apache Hive 4.0 镜像

Apache Hive 4.0 的 Docker 镜像有更好的配置支持：
- `SERVICE_NAME=metastore` 环境变量可正确启动 Metastore
- `SERVICE_NAME=hiveserver2` 可正确启动 HiveServer2
- 支持 `DB_DRIVER=postgres` 自动使用 PostgreSQL

```yaml
hive-metastore:
  image: apache/hive:4.0.0
  environment:
    SERVICE_NAME: metastore
    DB_DRIVER: postgres
    SERVICE_OPTS: >-
      -Djavax.jdo.option.ConnectionURL=jdbc:postgresql://...
```

### 方案 C：本地构建自定义镜像

基于 BDE 镜像，修改 entrypoint 脚本，强制 schematool 使用 PostgreSQL。

---

## 七、本地项目文件状态

### 7.1 已归档（旧项目）

```
~/archive/bilibili-v1/          ← 旧项目全部文件
  ├── crawler/                  ← Python 爬虫
  ├── backend/                  ← Rust 后端
  ├── frontend/                 ← React 前端
  ├── etl/                      ← ETL 脚本
  ├── scripts/                  ← 部署脚本
  └── docs/                     ← 文档
```

### 7.2 新项目（当前仓库）

```
/Volumes/code/GIT/Bilibili-Hot-Video-Analysis-Platform/
├── docs/
│   ├── superpowers/specs/2026-06-07-dashboard-v2-design.md    ← 设计文档
│   └── superpowers/plans/2026-06-07-dashboard-v2-implementation.md  ← 实现计划
├── crawler/          ← 已初始化，包含 Cargo.toml + 骨架代码
├── backend/          ← 已初始化，包含 Cargo.toml + 骨架代码
├── frontend/         ← 已初始化，包含 React + ECharts + Tailwind
├── etl/              ← 已初始化，包含 Python ETL 骨架
└── scripts/          ← 已初始化，包含 DDL + 部署脚本
```

### 7.3 WSL 状态

```
~/hadoop-cluster/              ← Docker Compose 配置目录
~/output/                      ← 爬虫数据（13轮快照，已清理旧项目后保留）
~/crawler/                     ← Python 爬虫代码（已清理）
```

---

## 八、WSL 已清理的内容

| 目录/服务 | 说明 | 状态 |
|-----------|------|------|
| ~/backend/ | Rust 后端 | ✅ 已删除 |
| ~/frontend/ | 前端静态文件 | ✅ 已删除 |
| ~/mapreduce/ | MapReduce 作业 | ✅ 已删除 |
| ~/scripts/ | 部署脚本 | ✅ 已删除 |
| ~/project/ | 项目文件 | ✅ 已删除 |
| ~/qemu/ | QEMU/libvirt VM (4.2G) | ✅ 已删除 |
| bilibili-backend.service | systemd 服务 | ✅ 已禁用删除 |
| crawler.service | systemd 服务 | ✅ 已禁用删除 |
| Hadoop 集群 VM | master/worker1/worker2 | ✅ 已关闭删除 |

---

## 九、关键配置信息

| 项目 | 值 |
|------|-----|
| WSL SSH 密码 | 902538 |
| MySQL root 密码 | Hive@123456 |
| MySQL 数据库 | bilibili |
| PostgreSQL (Hive Metastore) | 用户: hive, 密码: hive, 数据库: metastore |
| HDFS NameNode | hdfs://namenode:9000 |
| Spark Master | spark://spark-master:7077 |
| Docker 网络 | hadoop-cluster_bigdata_net |

---

## 十、下一步行动

1. **解决 Hive Schema 初始化问题**（阻塞项）
   - 尝试方案 A：手动执行 PostgreSQL DDL
   - 或尝试方案 B：使用 apache/hive:4.0.0

2. **验证 Hive 端到端**
   - beeline 连接 → SHOW DATABASES → CREATE TABLE → SELECT

3. **继续爬虫 Rust 重写**
   - 参考 ~/crawler/ 中的 Python 代码逻辑

4. **继续前端开发**
   - 根据设计文档实现各图表组件

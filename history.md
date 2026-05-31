# Hadoop 集群搭建历史记录

## 2026-05-30 20:30 - SSH 隧道搭建

### 目标
将 WSL Ubuntu (inkpear@192.168.31.75) 通过反向隧道投射到 jd_ecs 的 9822 端口，Mac 可通过 `ssh UbuntuDesktop` 直连。

### 关键操作
```bash
# WSL 生成 ed25519 密钥
ssh-keygen -t ed25519 -C 'wsl-ubuntu' -f ~/.ssh/id_ed25519 -N ''

# 公钥部署到 jd_ecs
echo 'ssh-ed25519 ...' >> ~/.ssh/authorized_keys

# 安装 autossh 并配置反向隧道 systemd 服务
sudo apt-get install -y autossh
# 创建 /etc/systemd/system/reverse-tunnel.service
# ExecStart=/usr/bin/autossh -M 0 -N -o ServerAliveInterval=30 -R 9822:localhost:22 -p 9722 root@111.228.34.34
sudo systemctl enable reverse-tunnel
sudo systemctl start reverse-tunnel

# Mac SSH 配置 ~/.ssh/config
# Host UbuntuDesktop
#     HostName 111.228.34.34
#     Port 9822
#     User inkpear
#     IdentityFile ~/.ssh/id_rsa
```

### 结果
反向隧道持久化运行，Mac 通过 `ssh UbuntuDesktop` 可直连 WSL。

### 验证
```bash
ssh UbuntuDesktop "echo UBUNTU-DESKTOP-OK"
# -> UBUNTU-DESKTOP-OK
```

### 问题与处理
- WSL 关闭终端后反向隧道中断，需在 WSL 终端内执行 `sudo systemctl restart reverse-tunnel`。
- 已关闭 WSL 密码认证，仅允许密钥登录。

---

## 2026-05-30 20:40 - QEMU 安装与 KVM 验证

### 目标
在 WSL Ubuntu 上安装 QEMU 并验证 KVM 可用。

### 关键操作
```bash
sudo apt-get install -y qemu-system-x86 qemu-utils
sudo usermod -aG kvm inkpear
```

### 结果
- QEMU 10.2.1 安装完成
- KVM 可用 (`/dev/kvm` 存在，`kvm-ok` 通过)
- CPU: Intel Ultra 7 265K, 20 核, 15GB RAM

### 验证
```bash
qemu-system-x86_64 --version
kvm-ok
# -> INFO: /dev/kvm exists / KVM acceleration can be used
```

---

## 2026-05-30 21:00 - CentOS 7 自动安装尝试（失败）

### 目标
使用 kickstart 自动化安装 CentOS 7 base 镜像。

### 问题
1. kickstart 写死静态 IP 192.168.100.10，QEMU SLIRP 网段 10.0.2.0/24 不通
2. 安装脚本 `-kernel/-initrd` 永远加载安装器内核，安装完成后重启又进入 anaconda
3. kickstart 中 `wget` 包不在 Minimal ISO 中，导致 anaconda 交互卡住
4. `pkill -f qemu-system-x86_64` 匹配当前 shell 命令导致远程会话被杀

### 教训
- kickstart 应使用 DHCP
- 安装脚本应拆分为 install/boot 两个脚本
- 安装完成后应 `shutdown` 而非 `reboot`
- Minimal ISO 需 `%packages --ignoremissing`

---

## 2026-05-30 22:10 - 手动安装 CentOS 7 成功

### 目标
通过 VNC 手动安装 CentOS 7 Minimal 到 QEMU 虚拟机。

### 关键操作
```bash
# 创建 20G qcow2 磁盘
qemu-img create -f qcow2 ~/qemu/vms/centos7-manual.qcow2 20G

# 启动 QEMU，VNC 监听 127.0.0.1:5901，密码 hadoop12
qemu-system-x86_64 \
  -enable-kvm -cpu host -m 2048 -smp 2 \
  -audio none \
  -object secret,id=vncpass,data=hadoop12 \
  -drive file=centos7-manual.qcow2,format=qcow2,if=virtio \
  -cdrom CentOS-7-x86_64-Minimal-2009.iso \
  -boot d \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -vnc 127.0.0.1:1,password-secret=vncpass

# Mac 端口转发
ssh -L 5901:127.0.0.1:5901 UbuntuDesktop
# VNC 连接 vnc://127.0.0.1:5901 密码 hadoop12
```

### 安装选项
- Language: English (United States)
- Timezone: Asia/Shanghai
- Installation Destination: 20G VirtIO 自动分区
- Network: DHCP 打开
- Root Password: hadoop123
- 用户 hadoop: hadoop123，administrator

### 结果
CentOS 7 Minimal 安装完成，系统可从磁盘启动。

### 问题
macOS Screen Sharing 不接受 VNC None 认证，需显式设置 `password-secret=vncpass`。

---

## 2026-05-30 22:20 - libvirt 安装与接管

### 目标
用 libvirt 统一管理 QEMU 虚拟机。

### 关键操作
```bash
# 安装 libvirt 全套
sudo apt-get install -y libvirt-daemon-system libvirt-clients virtinst \
  dnsmasq-base bridge-utils libosinfo-bin

# 安装 libguestfs（用于离线修改磁盘）
sudo apt-get install -y libguestfs-tools linux-image-virtual

# 启动服务
sudo systemctl enable --now libvirtd virtlogd virtlockd

# 用户加入 libvirt/kvm 组
sudo usermod -aG libvirt,kvm inkpear

# 启用 default NAT 网络
sudo virsh net-start default
sudo virsh net-autostart default
```

### 结果
- libvirt 12.0.0 安装完成
- `virsh qemu:///system version` 正常
- default 网络 active + autostart

### 问题
- WSL 无 `/boot` 内核，libguestfs supermin 报错
- 安装 `linux-image-virtual` 后解决

---

## 2026-05-30 22:30 - VM 导入 libvirt 并修复 SSH

### 目标
将手动安装的 qcow2 导入 libvirt，修复 SSH 密码认证。

### 关键操作
```bash
# 复制磁盘到 libvirt 标准目录
sudo install -o libvirt-qemu -g libvirt-qemu -m 0640 \
  ~/qemu/vms/centos7-manual.qcow2 /var/lib/libvirt/images/centos7-manual.qcow2

# 导入为 libvirt VM
sudo virt-install --connect qemu:///system --name centos7-manual \
  --memory 2048 --vcpus 2 --cpu host \
  --disk path=/var/lib/libvirt/images/centos7-manual.qcow2,format=qcow2,bus=virtio \
  --os-variant centos7.0 --network network=default,model=virtio \
  --graphics vnc,listen=127.0.0.1,password=hadoop12 \
  --noautoconsole --import

# 离线修复密码和 SSH 配置
sudo virt-customize -a /var/lib/libvirt/images/centos7-manual.qcow2 \
  --root-password password:hadoop123 \
  --password hadoop:password:hadoop123 \
  --run-command 'sed -i "s/^#\\?PasswordAuthentication .*/PasswordAuthentication yes/" /etc/ssh/sshd_config' \
  --run-command 'sed -i "s/^#\\?PermitRootLogin .*/PermitRootLogin yes/" /etc/ssh/sshd_config' \
  --run-command 'systemctl enable sshd' \
  --run-command 'systemctl enable NetworkManager'

# 重新启动
sudo virsh start centos7-manual
```

### 结果
- VM 由 libvirt 管理，IP: `192.168.122.55`
- root/hadoop 密码均重置为 `hadoop123`
- SSH 密码登录验证成功

### 验证
```bash
sshpass -p hadoop123 ssh root@192.168.122.55 "echo login-ok; whoami; hostname"
# -> login-ok / root / centos01

sshpass -p hadoop123 ssh hadoop@192.168.122.55 "echo login-ok; whoami; hostname"
# -> login-ok / hadoop / centos01
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| libguestfs supermin 找不到内核 | 安装 `linux-image-virtual` |
| macOS VNC 强制密码认证 | QEMU 加 `-object secret` + `password-secret` |
| SSH 密码 hadoop123 登录失败 | virt-customize 离线重置密码 |

---

## 2026-05-30 22:40 - libvirt 自定义网络 hadoop-net

### 目标
创建 Hadoop 三节点专用 NAT 网络，固定 IP 绑定。

### 关键操作
```bash
# 定义网络 XML
virsh net-define /tmp/hadoop-net.xml
virsh net-start hadoop-net
virsh net-autostart hadoop-net
```

### 网络配置
```text
名称: hadoop-net
网段: 192.168.100.0/24
网关: 192.168.100.1
DHCP 静态绑定:
  master  52:54:00:aa:00:10 -> 192.168.100.10
  worker1 52:54:00:aa:00:11 -> 192.168.100.11
  worker2 52:54:00:aa:00:12 -> 192.168.100.12
```

### 结果
hadoop-net active + autostart。

### 验证
```bash
virsh net-list --all
# hadoop-net   active   yes   yes
```

---

## 2026-05-30 22:44 - CentOS 7 模板制作

### 目标
基于 centos7-manual 制作统一模板 centos7-base，预装 JDK 和工具。

### 关键操作
```bash
# 复制磁盘
cp centos7-manual.qcow2 centos7-base.qcow2

# virt-customize 离线配置
virt-customize -a centos7-base.qcow2 \
  --hostname hadoop-base \
  --timezone Asia/Shanghai \
  --root-password password:hadoop123 \
  --password hadoop:password:hadoop123 \
  --run-command "sed -i 's|^mirrorlist=|#mirrorlist=|g' /etc/yum.repos.d/CentOS-*.repo" \
  --run-command "sed -i 's|^#baseurl=http://mirror.centos.org|baseurl=http://vault.centos.org|g' /etc/yum.repos.d/CentOS-*.repo" \
  --run-command "yum install -y java-1.8.0-openjdk java-1.8.0-openjdk-devel vim net-tools wget curl rsync openssh-clients ntp" \
  --run-command "systemctl disable firewalld" \
  --run-command "sed -i s/^SELINUX=enforcing/SELINUX=disabled/ /etc/selinux/config" \
  --run-command "echo '192.168.100.10 master' >> /etc/hosts" \
  --run-command "echo '192.168.100.11 worker1' >> /etc/hosts" \
  --run-command "echo '192.168.100.12 worker2' >> /etc/hosts" \
  --run-command "systemctl enable sshd NetworkManager"
```

### 结果
- JDK 1.8.0_412 安装成功
- SELinux disabled
- firewalld disabled
- /etc/hosts 预配置三节点
- sshd/NetworkManager enabled

### 验证
```bash
virt-customize -a centos7-base.qcow2 --run-command "java -version 2>&1"
# openjdk version "1.8.0_412"
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| CentOS 7 mirrorlist.centos.org 已下线 | 切换 baseurl 到 vault.centos.org |

---

## 2026-05-30 22:50 - 三节点 VM 克隆与启动

### 目标
从 centos7-base 克隆 master/worker1/worker2，配置到 hadoop-net。

### 关键操作
```bash
# 复制磁盘
cp centos7-base.qcow2 master.qcow2
cp centos7-base.qcow2 worker1.qcow2
cp centos7-base.qcow2 worker2.qcow2

# 创建 VM
virt-install --name master --memory 4096 --vcpus 2 --cpu host-model \
  --disk path=master.qcow2,format=qcow2,bus=virtio \
  --network network=hadoop-net,model=virtio,mac=52:54:00:aa:00:10 \
  --graphics vnc,listen=127.0.0.1,password=hadoop12 \
  --noautoconsole --import

virt-install --name worker1 --memory 2048 --vcpus 2 --cpu host-model \
  --disk path=worker1.qcow2,format=qcow2,bus=virtio \
  --network network=hadoop-net,model=virtio,mac=52:54:00:aa:00:11 \
  --graphics vnc,listen=127.0.0.1,password=hadoop12 \
  --noautoconsole --import

virt-install --name worker2 --memory 2048 --vcpus 2 --cpu host-model \
  --disk path=worker2.qcow2,format=qcow2,bus=virtio \
  --network network=hadoop-net,model=virtio,mac=52:54:00:aa:00:12 \
  --graphics vnc,listen=127.0.0.1,password=hadoop12 \
  --noautoconsole --import
```

### 结果
| 节点 | IP | 内存 | CPU | 状态 |
|---|---|---|---|---|
| master | 192.168.100.10 | 4G | 2 | running |
| worker1 | 192.168.100.11 | 2G | 2 | running |
| worker2 | 192.168.100.12 | 2G | 2 | running |

### 验证
```bash
# DHCP 租约
virsh net-dhcp-leases hadoop-net
# master  192.168.100.10
# worker1 192.168.100.11
# worker2 192.168.100.12

# SSH 连通性
for ip in 192.168.100.10 192.168.100.11 192.168.100.12; do
  sshpass -p hadoop123 ssh root@$ip "hostname; java -version 2>&1 | head -1"
done
# master / openjdk 1.8.0_412
# worker1 / openjdk 1.8.0_412
# worker2 / openjdk 1.8.0_412
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| centos7-base 未定义为 libvirt 域 | 改用 cp 磁盘 + virt-install --import |
| virt-clone 找不到源域 | 同上 |

---

## 2026-05-30 22:50 - Hadoop 用户与 SSH 免密配置

### 目标
在所有节点创建 hadoop 用户，配置 master 到所有节点的 SSH 免密登录。

### 关键操作
```bash
# 所有节点创建 hadoop 用户
useradd -m -s /bin/bash hadoop
echo hadoop:hadoop123 | chpasswd
echo "hadoop ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/hadoop

# master 生成密钥
su - hadoop -c "ssh-keygen -t rsa -P '' -f ~/.ssh/id_rsa"

# 分发公钥到所有节点
su - hadoop -c "ssh-copy-id hadoop@master"
su - hadoop -c "ssh-copy-id hadoop@worker1"
su - hadoop -c "ssh-copy-id hadoop@worker2"
```

### 结果
从 master 以 hadoop 用户 SSH 到 master/worker1/worker2 均无需密码。

### 验证
```bash
su - hadoop -c "ssh master hostname"    # master
su - hadoop -c "ssh worker1 hostname"   # worker1
su - hadoop -c "ssh worker2 hostname"   # worker2
```

---

## 2026-05-30 22:55 - Hadoop 3.3.6 安装与配置

### 目标
在 master 下载 Hadoop 3.3.6，配置 HDFS/YARN，分发到 workers。

### 关键操作
```bash
# master 下载安装
wget https://archive.apache.org/dist/hadoop/common/hadoop-3.3.6/hadoop-3.3.6.tar.gz
tar -xzf hadoop-3.3.6.tar.gz -C /opt/hadoop --strip-components=1

# 配置文件
core-site.xml:  fs.defaultFS=hdfs://master:9000
hdfs-site.xml:  dfs.replication=2
yarn-site.xml:  yarn.resourcemanager.hostname=master
mapred-site.xml: mapreduce.framework.name=yarn
workers:        master, worker1, worker2

# 分发到 workers
rsync -az /opt/hadoop/ hadoop@worker1:/opt/hadoop/
rsync -az /opt/hadoop/ hadoop@worker2:/opt/hadoop/

# 格式化并启动
hdfs namenode -format -force
start-dfs.sh
start-yarn.sh
```

### 结果
Hadoop 3.3.6 安装完成，集群启动成功。

### 验证
```bash
jps  # master 上应有 NameNode, DataNode, SecondaryNameNode, ResourceManager, NodeManager

hdfs dfsadmin -report
# Configured Capacity: 50.96 GB
# Present Capacity: 41.04 GB
# 3 DataNodes

yarn node -list
# Total Nodes:3
# master:RUNNING
# worker1:RUNNING
# worker2:RUNNING

hdfs dfs -mkdir -p /data/bilibili/ods
hdfs dfs -ls /
# drwxr-xr-x - hadoop supergroup 0 /data
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| CentOS 7 SSH 不支持 `accept-new` | 改用 `StrictHostKeyChecking=no` |
| rsync 权限不足 | 先在 worker 上 `mkdir /opt/hadoop && chown hadoop:hadoop` |
| CentOS 7 mirrorlist 已下线 | 切换 baseurl 到 vault.centos.org |

---

## 2026-05-30 23:00 - MySQL 8.0.46 安装

### 目标
在 master 上安装 MySQL 8，作为 Hive metastore 后端和后续 ADS 存储。

### 关键操作
```bash
# 添加 MySQL 8 官方 repo
rpm --import https://repo.mysql.com/RPM-GPG-KEY-mysql-2023
cat > /etc/yum.repos.d/mysql-community.repo << 'EOF'
[mysql80-community]
name=MySQL 8.0 Community Server
baseurl=https://repo.mysql.com/yum/mysql-8.0-community/el/7/$basearch/
enabled=1
gpgcheck=1
gpgkey=https://repo.mysql.com/RPM-GPG-KEY-mysql-2022
       https://repo.mysql.com/RPM-GPG-KEY-mysql-2023
EOF

yum install -y mysql-community-server
systemctl start mysqld && systemctl enable mysqld

# 重置 root 密码，创建数据库和用户
mysql -u root -p<临时密码> < setup_mysql.sql
# setup_mysql.sql:
#   SET GLOBAL validate_password.policy=LOW;
#   ALTER USER 'root'@'localhost' IDENTIFIED BY 'Root@123456';
#   CREATE DATABASE hive_metastore;
#   CREATE DATABASE bilibili;
#   CREATE USER 'hive'@'%' IDENTIFIED BY 'Hive@123456';
#   GRANT ALL ON hive_metastore.* TO 'hive'@'%';
#   GRANT ALL ON bilibili.* TO 'hive'@'%';
```

### 结果
- MySQL 8.0.46 安装并运行
- root 密码: Root@123456
- hive 用户密码: Hive@123456
- 数据库: hive_metastore, bilibili

### 验证
```bash
mysql -u hive -pHive@123456 -e "SHOW DATABASES;"
# hive_metastore, bilibili
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| GPG key 不匹配 (8.0.46) | 添加 2023 key 到 gpgkey 列表 |
| 临时密码过期 | 用 `mysqld --init-file` 重置 |
| 密码策略限制 | `SET GLOBAL validate_password.policy=LOW` |

---

## 2026-05-30 23:10 - Hive 3.1.3 安装（多节点）

### 目标
在 master 安装 Hive Metastore + HiveServer2，分发到所有节点。

### 部署架构
| 节点 | 角色 |
|---|---|
| master | Metastore (9083) + HiveServer2 (10000) + CLI |
| worker1 | YARN worker (Hive JARs 分发到位) |
| worker2 | YARN worker (Hive JARs 分发到位) |

### 关键操作
```bash
# master 下载安装
wget https://archive.apache.org/dist/hive/hive-3.1.3/apache-hive-3.1.3-bin.tar.gz
tar -xzf ... -C /opt/hive

# 配置 hive-site.xml (MySQL metastore)
# javax.jdo.option.ConnectionURL = jdbc:mysql://master:3306/hive_metastore
# javax.jdo.option.ConnectionUserName = hive
# javax.jdo.option.ConnectionPassword = Hive@123456

# MySQL JDBC driver
cp mysql-connector-j-8.0.33.jar /opt/hive/lib/

# 初始化 metastore schema
schematool -initSchema -dbType mysql

# 分发到 workers
rsync -az /opt/hive/ hadoop@worker1:/opt/hive/
rsync -az /opt/hive/ hadoop@worker2:/opt/hive/

# 启动
hive --service metastore &
hive --service hiveserver2 &
```

### 结果
- Hive 3.1.3 安装完成
- Metastore schema 初始化成功
- 分发到 worker1/worker2

### 验证
```bash
hive -e "SHOW DATABASES;"
# default
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| hadoop 用户无 /opt 写权限 | root 执行安装，chown 给 hadoop |
| rsync 从 WSL 而非 master 执行 | 写脚本上传到 master 执行 |

---

## 2026-05-30 23:15 - Spark 3.5.3 安装（多节点）

### 目标
在 master 安装 Spark Master + History Server，worker1/worker2 运行 Worker。

### 部署架构
| 节点 | 角色 |
|---|---|
| master | Master (7077) + Worker + History Server (18080) |
| worker1 | Worker |
| worker2 | Worker |

### 关键操作
```bash
# master 下载安装
wget https://archive.apache.org/dist/spark/spark-3.5.3/spark-3.5.3-bin-hadoop3.tgz
tar -xzf ... -C /opt/spark

# 配置 spark-env.sh
# SPARK_MASTER_HOST=master, JAVA_HOME, HADOOP_HOME

# 配置 workers
echo -e "master\nworker1\nworker2" > /opt/spark/conf/workers

# 分发到 workers
rsync -az /opt/spark/ hadoop@worker1:/opt/spark/
rsync -az /opt/spark/ hadoop@worker2:/opt/spark/

# 启动
start-all.sh
start-history-server.sh
```

### 结果
- Spark 3.5.3 安装完成
- 3 个 Worker 节点运行

### 验证
```bash
# 所有节点 JPS
master:   NameNode, DataNode, SecondaryNameNode, ResourceManager, NodeManager, Master, Worker, HistoryServer, RunJar(Hive)
worker1:  DataNode, NodeManager, Worker
worker2:  DataNode, NodeManager, Worker

# Spark Shell
spark-shell --master spark://master:7077
# sc.parallelize(1 to 100).reduce(_ + _) = 5050

# Spark SQL
spark-sql --master spark://master:7077 -e "SHOW DATABASES;"
# default
```

### 问题与处理
| 问题 | 解决 |
|---|---|
| rsync 权限不足 | root 在 worker 上预创建目录 |
| spark-defaults.sh 命名错误 | 应为 spark-defaults.conf，Spark 不读取 .sh |
| Spark 缺少 MySQL JDBC driver | 从 Hive lib 复制 mysql-connector-j 到 spark/jars |
| Spark 找不到 Hive metastore | 创建 hive-site.xml 软链接到 spark/conf |

---

## 2026-05-31 全天 — 爬虫架构重写与数据管线部署

### 目标
将爬虫从队列分摊模型重构为 per-video 事务模型，部署到 WSL 自动采集，打通 HDFS→Hive→Spark→MySQL 全链路。

### 重构内容

**模型变更**：
- 删除 `queue_comments`/`queue_danmaku` 两张队列表及全部 CRUD 方法
- 新增 `process_video()` 单视频事务编排器：/view → UP → 评论 → 弹幕
- 新增 `fail_count` 失败追踪（≥3 永久放弃）和 `discover_new_items` 返回 retry 列表
- 输出改为流式追加写入（除 rank_video 仍为批量 ~1000 行）
- tqdm 单条全局进度条 + `pbar.write()` 逐视频状态行
- systemd timer 从每 6h × 28 改为每天 1 轮 × 7 轮（`OnCalendar=daily`）

**数据减法**：
- 移除番剧(rid=13) — B站已下线
- 移除每周必看 — API 返回 code=-352

**Bug 修复**：
| 问题 | 解决 |
|---|---|
| `snapshot_id` 格式不一致（HHmmss vs HHmm） | 统一为 `yyyyMMdd_HHmm` |
| `deploy_crawler.sh` rsync 源路径错误 | `$SCRIPT_DIR` → `$CRAWLER_DIR` |
| description 合并到 rank_video 未生效 | 在 `process_video` 中正确定向修改 row 后再合并 |
| tqdm 4.67.3 + Python 3.13 兼容性 | `disable=not sys.stderr.isatty()` with screen TTY |
| WBI 签名 `-352` 风控失败 | Cookie 中 `;` 被 bash 解析为命令分隔符，改用 `grep -o \| sed` 提取 |
| 后台日志不实时写入 | screen 托管（TTY → 无缓冲问题） |

### 数据管线建设

**SQL 脚本**（部署在 `sql/`）：
- `hive_ods_ddl.sql` — 4 张 Hive 外部表（JsonSerDe）
- `spark_etl.sql` — ODS→DWD(5)→DWS(3)→ADS(7) 全链路
- `mysql_ads_ddl.sql` — 7 张 MySQL ADS 表

**管线脚本**（部署在 `scripts/`）：
- `upload_hdfs.sh` — NDJSON → HDFS（从 WSL 通过 sshpass 上传）
- `export_to_mysql.py` — PySpark JDBC 导出（TRUNCATE + INSERT）
- `run_pipeline.sh` — 日常 ETL 编排
- `setup_pipeline.sh` — 首次集群初始化
- `start_crawler.sh` — screen 会话启动脚本（正确提取 Cookie）

**集群部署验证**（在 master:192.168.100.10）：
```bash
# HDFS 目录
hdfs dfs -ls /data/bilibili/ods/     # rank_video, video_comment, video_danmaku, up_info

# Hive 表
hive -e "SHOW TABLES;"               # ods_*_raw × 4

# MySQL 表
mysql -u hive -pHive@123456 bilibili -e "SHOW TABLES;"
# ads_screen_overview, ads_hot_video_rank, ads_partition_compare,
# ads_video_lifecycle, ads_partition_daily, ads_up_summary, ads_tag_cloud

# Spark 连通
spark-sql --master spark://master:7077 -e "SHOW DATABASES;"  # bilibili, default

# YARN 节点
yarn node -list                     # 3 nodes RUNNING
```

### 部署状态

| 组件 | 位置 | 状态 |
|---|---|---|
| crawler | WSL `~/crawler/` | screen 会话运行中（14:37 启动） |
| systemd timer | WSL user-level | active，明天 00:00 触发 |
| Hive Metastore + HiveServer2 | master | RUNNING |
| Spark Master + 2 Workers | master + worker1/2 | RUNNING |
| MySQL 8.0.46 | master | RUNNING，数据库 bilibili 已就绪 |

### 关键设计决策

| 决策 | 理由 |
|---|---|
| 日结 vs 分摊 | 每天 1 轮贪心全量消费队列，保证每天数据完整，首日 ~7h 后续 <15min |
| per-video 事务 vs 分步队列 | 队列模型维护复杂，事务模型天然保证数据一致性 |
| UP 主独立提交 | 跨视频共享数据，不应因一个视频失败而丢弃 UP 信息 |
| screen 托管 vs systemd | screen 提供 TTY → tqdm 动态进度条；systemd 用于自动定时触发 |
| Cookie 提取 | `grep -o \| sed`，不能用 `xargs`（`;` 会被 bash 误解析） |

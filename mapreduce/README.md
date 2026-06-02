# 词频 MapReduce

`mapper.py` / `reducer.py` 用于 Hadoop Streaming 中文分词词频统计，结果写入 `ads_word_cloud`。

支持三种 source_type：
- `title`：标题词云（输入来自 `ads_hot_video_rank`）
- `comment`：评论词云（输入来自 `dwd_comment_info`）
- `danmaku`：弹幕词云（输入来自 `dwd_danmaku_info`）

输入每行格式：

```text
category<TAB>content
```

输出每行格式：

```text
category<TAB>word<TAB>word_count
```

mapper 会优先使用 `jieba`，节点没有 jieba 时自动退化为内置中文 n-gram 分词。

停用词文件：
- `stopwords.txt`：标题通用停用词
- `stopwords_comment.txt`：评论专用停用词（含口水词、表情词）
- `stopwords_danmaku.txt`：弹幕专用停用词（保留即时反应词）

用法：

```bash
SOURCE_TYPE=title bash scripts/run_tag_cloud_mr.sh
SOURCE_TYPE=comment bash scripts/run_tag_cloud_mr.sh
SOURCE_TYPE=danmaku bash scripts/run_tag_cloud_mr.sh
```

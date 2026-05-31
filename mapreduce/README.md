# 标题词频 MapReduce

`mapper.py` / `reducer.py` 用于 Hadoop Streaming 标题分词词频统计，结果写入 `ads_tag_cloud`。

输入每行格式：

```text
category<TAB>title
```

输出每行格式：

```text
category<TAB>tag<TAB>tag_count
```

mapper 会优先使用 `jieba`，节点没有 jieba 时自动退化为内置中文 n-gram 分词。

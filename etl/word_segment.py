import jieba
import re
from collections import Counter

class WordSegmenter:
    def __init__(self):
        # 停用词表
        self.stopwords = set([
            '的', '了', '在', '是', '我', '有', '和', '就', '不', '人',
            '都', '一', '一个', '上', '也', '很', '到', '说', '要', '去',
            '你', '会', '着', '没有', '看', '好', '自己', '这', '他', '她',
            '哈哈哈', '666', '来了', '好的', '可以', '什么', '怎么',
            '视频', '播放', '投稿', '热门', '合集', '完整版', '全程',
        ])
        
    def clean_text(self, text: str) -> str:
        """清洗文本"""
        # 移除emoji
        text = re.sub(r'[\U00010000-\U0010ffff]', '', text)
        # 移除特殊字符
        text = re.sub(r'[^\w\s]', '', text)
        return text.strip()
    
    def segment(self, text: str) -> list[str]:
        """分词并过滤"""
        text = self.clean_text(text)
        words = jieba.lcut(text)
        # 过滤停用词和短词
        words = [w for w in words if w not in self.stopwords and len(w) > 1]
        return words
    
    def segment_batch(self, texts: list[str]) -> Counter:
        """批量分词并统计词频"""
        counter = Counter()
        for text in texts:
            words = self.segment(text)
            counter.update(words)
        return counter
    
    def get_top_words(self, texts: list[str], top_n: int = 100) -> list[tuple[str, int]]:
        """获取TOP N高频词"""
        counter = self.segment_batch(texts)
        return counter.most_common(top_n)

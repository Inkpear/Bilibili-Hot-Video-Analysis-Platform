use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use jieba_rs::Jieba;
use regex::Regex;
use tracing::info;

use crate::error::Result;
use crate::models::{Comment, Danmaku, VideoDetail};

pub struct WordSegmenter {
    jieba: Jieba,
    stopwords: HashSet<String>,
    emoji_re: Regex,
    punct_re: Regex,
}

type WordEntry = (String, String, String, i64); // (bvid, word, category, count)

impl WordSegmenter {
    pub fn new(stopwords_path: &Path) -> Result<Self> {
        let content = fs::read_to_string(stopwords_path)?;
        let stopwords: HashSet<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        info!(count = stopwords.len(), "loaded stopwords");

        Ok(Self {
            jieba: Jieba::new(),
            stopwords,
            emoji_re: Regex::new(r"[\U00010000-\U0010ffff]").unwrap(),
            punct_re: Regex::new(r"[^\w\s]").unwrap(),
        })
    }

    fn clean_text(&self, text: &str) -> String {
        let text = self.emoji_re.replace_all(text, "");
        let text = self.punct_re.replace_all(&text, "");
        text.trim().to_string()
    }

    pub fn segment(&self, text: &str) -> Vec<String> {
        let cleaned = self.clean_text(text);
        self.jieba
            .cut(&cleaned, false)
            .into_iter()
            .map(|w| w.word.trim().to_string())
            .filter(|w| !w.is_empty() && !self.stopwords.contains(w) && w.chars().count() > 1)
            .collect()
    }

    fn segment_and_count(&self, texts: &[String]) -> Vec<(String, i64)> {
        let mut counter: HashMap<String, i64> = HashMap::new();
        for text in texts {
            for word in self.segment(text) {
                *counter.entry(word).or_insert(0) += 1;
            }
        }
        let mut pairs: Vec<_> = counter.into_iter().collect();
        pairs.sort_by_key(|b| std::cmp::Reverse(b.1));
        pairs.truncate(50);
        pairs
    }

    pub fn segment_by_bvid_titles(&self, videos: &[VideoDetail]) -> Vec<WordEntry> {
        let mut by_bvid: HashMap<&str, Vec<String>> = HashMap::new();
        let mut bvid_category: HashMap<&str, &str> = HashMap::new();
        for v in videos {
            by_bvid.entry(&v.bvid).or_default().push(v.title.clone());
            bvid_category
                .entry(&v.bvid)
                .or_insert(if v.category.is_empty() {
                    &v.sub_category
                } else {
                    &v.category
                });
        }

        let mut result = Vec::new();
        for (bvid, texts) in &by_bvid {
            let category = bvid_category.get(bvid).unwrap_or(&"").to_string();
            for (word, count) in self.segment_and_count(texts) {
                result.push((bvid.to_string(), word, category.clone(), count));
            }
        }
        result
    }

    pub fn segment_by_bvid_comments(&self, comments: &[Comment], videos: &[VideoDetail]) -> Vec<WordEntry> {
        let bvid_category: HashMap<&str, &str> = videos
            .iter()
            .map(|v| {
                (
                    v.bvid.as_str(),
                    if v.category.is_empty() {
                        v.sub_category.as_str()
                    } else {
                        v.category.as_str()
                    },
                )
            })
            .collect();

        let mut by_bvid: HashMap<&str, Vec<String>> = HashMap::new();
        for c in comments {
            by_bvid
                .entry(&c.bvid)
                .or_default()
                .push(c.content.clone());
        }

        let mut result = Vec::new();
        for (bvid, texts) in &by_bvid {
            let category = bvid_category.get(bvid).unwrap_or(&"").to_string();
            for (word, count) in self.segment_and_count(texts) {
                result.push((bvid.to_string(), word, category.clone(), count));
            }
        }
        result
    }

    pub fn segment_by_bvid_danmaku(
        &self,
        danmaku_list: &[Danmaku],
        videos: &[VideoDetail],
    ) -> Vec<WordEntry> {
        let bvid_category: HashMap<&str, &str> = videos
            .iter()
            .map(|v| {
                (
                    v.bvid.as_str(),
                    if v.category.is_empty() {
                        v.sub_category.as_str()
                    } else {
                        v.category.as_str()
                    },
                )
            })
            .collect();

        let mut by_bvid: HashMap<&str, Vec<String>> = HashMap::new();
        for d in danmaku_list {
            by_bvid
                .entry(&d.bvid)
                .or_default()
                .push(d.content.clone());
        }

        let mut result = Vec::new();
        for (bvid, texts) in &by_bvid {
            let category = bvid_category.get(bvid).unwrap_or(&"").to_string();
            for (word, count) in self.segment_and_count(texts) {
                result.push((bvid.to_string(), word, category.clone(), count));
            }
        }
        result
    }

    pub fn compute_trends(
        &self,
        title_words: &[WordEntry],
        comment_words: &[WordEntry],
        danmaku_words: &[WordEntry],
    ) -> Vec<(String, String, String, i64)> {
        // (word, snapshot_date placeholder, category, source_type, total_count)
        // We return (word, category, source_type, total_count) — snapshot_date filled by caller
        let sources: [(&str, &[WordEntry]); 3] = [
            ("title", title_words),
            ("comment", comment_words),
            ("danmaku", danmaku_words),
        ];

        let mut result = Vec::new();
        for (source_type, entries) in sources {
            let mut word_total: HashMap<&str, (i64, &str)> = HashMap::new();
            for (_bvid, word, category, count) in entries {
                let entry = word_total.entry(word.as_str()).or_insert((0, category));
                entry.0 += count;
            }
            let mut sorted: Vec<_> = word_total.into_iter().collect();
            sorted.sort_by_key(|b| std::cmp::Reverse(b.1 .0));
            for (word, (total, category)) in sorted.into_iter().take(200) {
                result.push((
                    word.to_string(),
                    category.to_string(),
                    source_type.to_string(),
                    total,
                ));
            }
        }
        result
    }
}

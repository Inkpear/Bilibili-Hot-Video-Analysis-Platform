import json
import os
import glob
from datetime import datetime
from db import Database
from word_segment import WordSegmenter


class DataProcessor:
    def __init__(self, date_str: str):
        self.date_str = date_str
        self.db = Database()
        self.output_dir = os.path.expanduser("~/output")

    def find_latest_snapshot(self) -> str:
        # Crawler creates directories as YYYYMMDD (no underscore)
        pattern = f"{self.output_dir}/{self.date_str}*"
        dirs = sorted(glob.glob(pattern))
        if not dirs:
            raise FileNotFoundError(f"No snapshot found for {self.date_str}")
        return dirs[-1]

    def load_video_details(self, snapshot_dir: str) -> list:
        videos = []
        video_dir = f"{snapshot_dir}/video_detail"
        if not os.path.isdir(video_dir):
            # Fallback: single file (legacy format)
            path = f"{snapshot_dir}/video_detail.ndjson"
            if os.path.exists(path):
                with open(path, 'r') as f:
                    for line in f:
                        if line.strip():
                            videos.append(json.loads(line))
            return videos
        for filepath in glob.glob(f"{video_dir}/*.ndjson"):
            with open(filepath, 'r') as f:
                for line in f:
                    if line.strip():
                        videos.append(json.loads(line))
        return videos

    def load_up_info(self, snapshot_dir: str) -> list:
        up_list = []
        up_dir = f"{snapshot_dir}/up_info"
        if not os.path.isdir(up_dir):
            # Fallback: single file (legacy format)
            path = f"{snapshot_dir}/up_info.ndjson"
            if os.path.exists(path):
                with open(path, 'r') as f:
                    for line in f:
                        if line.strip():
                            up_list.append(json.loads(line))
            return up_list
        for filepath in glob.glob(f"{up_dir}/*.ndjson"):
            with open(filepath, 'r') as f:
                for line in f:
                    if line.strip():
                        up_list.append(json.loads(line))
        seen = set()
        deduped = []
        for up in up_list:
            uid = up.get('up_id', '')
            if uid and uid not in seen:
                seen.add(uid)
                deduped.append(up)
        return deduped

    def load_comments(self, snapshot_dir: str) -> list:
        comments = []
        comments_dir = f"{snapshot_dir}/comments"
        if not os.path.isdir(comments_dir):
            return comments
        for filepath in glob.glob(f"{comments_dir}/*.ndjson"):
            with open(filepath, 'r') as f:
                for line in f:
                    if line.strip():
                        comments.append(json.loads(line))
        return comments

    def load_danmaku(self, snapshot_dir: str) -> list:
        danmaku_list = []
        danmaku_dir = f"{snapshot_dir}/danmaku"
        if not os.path.isdir(danmaku_dir):
            return danmaku_list
        for filepath in glob.glob(f"{danmaku_dir}/*.ndjson"):
            with open(filepath, 'r') as f:
                for line in f:
                    if line.strip():
                        danmaku_list.append(json.loads(line))
        return danmaku_list

    def _process_word_cloud_per_bvid(self, items, table: str, bvid_key='bvid', content_key='content'):
        segmenter = WordSegmenter()
        by_bvid = {}
        for item in items:
            bvid = item.get(bvid_key, '')
            text = item.get(content_key, '')
            if bvid and text:
                by_bvid.setdefault(bvid, []).append(text)

        all_words = []
        for bvid, texts in by_bvid.items():
            top_words = segmenter.get_top_words(texts, top_n=50)
            for word, count in top_words:
                all_words.append((bvid, word, count, ''))
        print(f"  {table}: {len(all_words)} word entries from {len(by_bvid)} bvids")
        if all_words:
            self.db.insert_word_cloud_batch(table, all_words, self.date_str)

    def run(self):
        snapshot_dir = self.find_latest_snapshot()
        print(f"Processing snapshot: {snapshot_dir}")

        # 1. Load data
        videos = self.load_video_details(snapshot_dir)
        print(f"Loaded {len(videos)} videos")
        up_list = self.load_up_info(snapshot_dir)
        if up_list:
            print(f"Loaded {len(up_list)} UP info records")
        comments = self.load_comments(snapshot_dir)
        print(f"Loaded {len(comments)} comments")
        danmaku_list = self.load_danmaku(snapshot_dir)
        print(f"Loaded {len(danmaku_list)} danmaku")

        # 2. Batch insert videos
        print("Inserting videos...")
        self.db.insert_videos_batch(videos, self.date_str)

        # 3. Batch insert interactions
        print("Inserting interactions...")
        interactions = []
        for v in videos:
            vc = v.get('view_count', 0)
            interactions.append((
                v['bvid'], self.date_str,
                v['like_count'] / vc if vc else 0,
                v['coin_count'] / vc if vc else 0,
                v['favorite_count'] / vc if vc else 0,
                v['reply_count'] / vc if vc else 0,
                v['share_count'] / vc if vc else 0,
            ))
        self.db.insert_interactions_batch(interactions)

        # 4. Batch insert UP info
        if up_list:
            print("Inserting UP info...")
            self.db.insert_up_info_batch(up_list, self.date_str)

        # 5. Comments + comment word cloud
        if comments:
            print("Inserting comments...")
            self.db.insert_comments_batch(comments, self.date_str)
            self._process_word_cloud_per_bvid(comments, 'word_cloud_comment')

        # 6. Danmaku + danmaku word cloud
        if danmaku_list:
            print("Inserting danmaku...")
            self.db.insert_danmaku_batch(danmaku_list, self.date_str)
            self._process_word_cloud_per_bvid(danmaku_list, 'word_cloud_danmaku')

        # 7. Title word cloud
        print("Processing title word cloud...")
        segmenter = WordSegmenter()
        title_words = []
        for v in videos:
            title = v.get('title', '')
            if title:
                cat = v.get('category', '') or v.get('sub_category', '')
                for word in segmenter.segment(title):
                    title_words.append((v['bvid'], word, 1, cat))
        print(f"  word_cloud_title: {len(title_words)} entries")
        if title_words:
            self.db.insert_word_cloud_batch('word_cloud_title', title_words, self.date_str)

        # 8. Word trends (global)
        print("Processing word trends...")
        trends = []
        title_texts = [v.get('title', '') for v in videos if v.get('title')]
        if title_texts:
            counter = segmenter.segment_batch(title_texts)
            for word, count in counter.most_common(200):
                trends.append((word, '', 'title', count))
        comment_texts = [c.get('content', '') for c in comments if c.get('content')]
        if comment_texts:
            counter = segmenter.segment_batch(comment_texts)
            for word, count in counter.most_common(200):
                trends.append((word, '', 'comment', count))
        danmaku_texts = [d.get('content', '') for d in danmaku_list if d.get('content')]
        if danmaku_texts:
            counter = segmenter.segment_batch(danmaku_texts)
            for word, count in counter.most_common(200):
                trends.append((word, '', 'danmaku', count))
        if trends:
            self.db.insert_word_trend_batch(trends, self.date_str)
            print(f"  word_trend: {len(trends)} entries")

        # 9. Snapshot date
        self.db.insert_snapshot_date(self.date_str, len(videos))
        self.db.close()
        print("ETL completed!")

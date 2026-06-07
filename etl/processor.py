# etl/processor.py
import json
import os
import glob
from datetime import datetime
from db import Database

class DataProcessor:
    def __init__(self, date_str: str):
        self.date_str = date_str
        self.db = Database()
        self.output_dir = os.path.expanduser("~/output")
        
    def find_latest_snapshot(self) -> str:
        """找到指定日期的最新快照目录"""
        pattern = f"{self.output_dir}/{self.date_str}_*"
        dirs = sorted(glob.glob(pattern))
        if not dirs:
            raise FileNotFoundError(f"No snapshot found for {self.date_str}")
        return dirs[-1]
    
    def load_video_details(self, snapshot_dir: str) -> list:
        """加载视频详情"""
        path = f"{snapshot_dir}/video_detail.ndjson"
        videos = []
        with open(path, 'r') as f:
            for line in f:
                if line.strip():
                    videos.append(json.loads(line))
        return videos
    
    def calculate_interaction_rates(self, video: dict) -> dict:
        """计算互动指标"""
        view_count = video.get('view_count', 0)
        if view_count == 0:
            return {
                'like_rate': 0, 'coin_rate': 0, 'fav_rate': 0,
                'reply_rate': 0, 'share_rate': 0
            }
        return {
            'like_rate': video['like_count'] / view_count,
            'coin_rate': video['coin_count'] / view_count,
            'fav_rate': video['favorite_count'] / view_count,
            'reply_rate': video['reply_count'] / view_count,
            'share_rate': video['share_count'] / view_count,
        }
    
    def run(self):
        """执行ETL流程"""
        snapshot_dir = self.find_latest_snapshot()
        print(f"Processing snapshot: {snapshot_dir}")
        
        # 1. 加载视频数据
        videos = self.load_video_details(snapshot_dir)
        print(f"Loaded {len(videos)} videos")
        
        # 2. 写入hot_video表
        for video in videos:
            self.db.insert_video(video, self.date_str)
        
        # 3. 计算并写入互动指标
        for video in videos:
            rates = self.calculate_interaction_rates(video)
            self.db.insert_interaction(video['bvid'], self.date_str, rates)
        
        # 4. 处理评论和弹幕
        # TODO
        
        print("ETL completed!")

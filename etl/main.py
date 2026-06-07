# etl/main.py
import sys
from datetime import datetime, timedelta
from processor import DataProcessor
from word_segment import WordSegmenter
from db import Database

def main():
    # 默认处理前一天的数据
    if len(sys.argv) > 1:
        date_str = sys.argv[1]
    else:
        yesterday = datetime.now() - timedelta(days=1)
        date_str = yesterday.strftime("%Y%m%d")
    
    print(f"Processing data for {date_str}...")
    
    # 1. 找到最新的快照目录
    # 2. 读取并清洗数据
    # 3. 计算衍生指标
    # 4. 分词计算词云
    # 5. 写入MySQL
    
    processor = DataProcessor(date_str)
    processor.run()

if __name__ == "__main__":
    main()

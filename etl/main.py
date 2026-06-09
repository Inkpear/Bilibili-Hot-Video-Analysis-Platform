# etl/main.py
import argparse
from datetime import datetime, timedelta
from processor import DataProcessor


def main():
    parser = argparse.ArgumentParser(description='Bilibili ETL Pipeline')
    parser.add_argument('date', nargs='?', help='Date string YYYYMMDD (default: yesterday)')
    parser.add_argument('--data-dir', help='Override output directory (default: ~/output)')
    args = parser.parse_args()

    if args.date:
        date_str = args.date
    else:
        yesterday = datetime.now() - timedelta(days=1)
        date_str = yesterday.strftime("%Y%m%d")

    print(f"Processing data for {date_str}...")

    processor = DataProcessor(date_str)
    if args.data_dir:
        processor.output_dir = args.data_dir

    processor.run()


if __name__ == "__main__":
    main()

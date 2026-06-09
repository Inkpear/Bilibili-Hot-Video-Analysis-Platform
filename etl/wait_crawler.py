import sqlite3, sys, time
from datetime import datetime

state_db = '/home/inkpear/crawler-v2/crawler_state.db'
yesterday = sys.argv[1] if len(sys.argv) > 1 else datetime.now().strftime('%Y%m%d')
max_wait = 86400  # 24h
waited = 0

print(f'[{datetime.now()}] Waiting for crawler full crawl on {yesterday}')

while waited < max_wait:
    try:
        conn = sqlite3.connect(state_db)
        row = conn.execute(
            "SELECT COUNT(*) FROM snapshot_log WHERE snapshot_id LIKE ? AND status='ok'",
            (f'{yesterday}%',)
        ).fetchone()
        conn.close()
        if row and row[0] > 0:
            print(f'[{datetime.now()}] Crawler done for {yesterday}')
            sys.exit(0)
    except Exception as e:
        print(f'[{datetime.now()}] DB error: {e}')
    
    if waited % 600 == 0:
        print(f'[{datetime.now()}] Waiting... {waited}s')
    time.sleep(60)
    waited += 60

print(f'[{datetime.now()}] TIMEOUT after 24h')
sys.exit(1)

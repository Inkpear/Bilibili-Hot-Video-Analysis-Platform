use bilibili_crawler::storage::NdjsonWriter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestItem {
    id: i64,
    name: String,
}

#[test]
fn test_ndjson_roundtrip() {
    let tmp = std::env::temp_dir().join(format!("ndjson_test_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("create tmp");

    let writer = NdjsonWriter::new(tmp.to_str().unwrap());
    let snap_dir = writer.create_snapshot_dir().expect("create snapshot dir");
    assert!(snap_dir.exists());
    assert!(snap_dir.join("comments").exists());
    assert!(snap_dir.join("danmaku").exists());

    // Write
    let items = vec![
        TestItem { id: 1, name: "one".into() },
        TestItem { id: 2, name: "two".into() },
    ];
    let path = snap_dir.join("test.ndjson");
    writer.write_ndjson(&path, &items).expect("write");

    // Read back
    let mut content = String::new();
    fs::File::open(&path).expect("open").read_to_string(&mut content).expect("read");
    let lines: Vec<TestItem> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("parse"))
        .collect();
    assert_eq!(lines, items);

    // Append
    let more = vec![TestItem { id: 3, name: "three".into() }];
    writer.append_ndjson(&path, &more).expect("append");

    let mut content2 = String::new();
    fs::File::open(&path).expect("open").read_to_string(&mut content2).expect("read");
    let lines2: Vec<TestItem> = content2
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("parse"))
        .collect();
    assert_eq!(lines2.len(), 3);

    // Cleanup
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_snapshot_dir_naming() {
    let tmp = std::env::temp_dir().join(format!("ndjson_naming_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("create tmp");

    let writer = NdjsonWriter::new(tmp.to_str().unwrap());
    let snap_dir = writer.create_snapshot_dir().expect("create");

    let dir_name = snap_dir.file_name().unwrap().to_string_lossy();
    // Format: YYYYMMDD_HHMM
    assert_eq!(dir_name.len(), 13, "dir name should be YYYYMMDD_HHMM (13 chars)");
    assert!(dir_name.contains('_'), "should contain underscore separator");

    let _ = fs::remove_dir_all(&tmp);
}

use bilibili_crawler::state::StateManager;

#[test]
fn test_full_video_lifecycle() {
    let db_path = ":memory:";
    let state = StateManager::new(db_path).expect("create state manager");

    let bvid = "BV1TEST001";
    let snapshot = "20260608_0000";

    // Insert new video
    let is_new = state
        .insert_video(bvid, "Test Title", "动画", "综合", 123, 456, 120, 500, "999", snapshot)
        .expect("insert video");
    assert!(is_new, "first insert should return true");

    // Duplicate insert
    let is_new2 = state
        .insert_video(bvid, "Test Title", "动画", "综合", 123, 456, 120, 500, "999", snapshot)
        .expect("insert video again");
    assert!(!is_new2, "duplicate insert should return false");

    // Mark detail done
    state.mark_detail_done(bvid).expect("mark detail");

    // Check not done yet
    assert!(!state.video_done(bvid).expect("video_done"));

    // Mark comments and danmaku
    state.mark_comments_done(bvid).expect("mark comments");
    state.mark_danmaku_done(bvid).expect("mark danmaku");

    // Now done
    assert!(state.video_done(bvid).expect("video_done"));

    // Verify info
    let info = state.get_video_info(bvid).expect("get info").expect("has info");
    assert_eq!(info.bvid, bvid);
    assert!(info.has_detail);
    assert!(info.has_comments);
    assert!(info.has_danmaku);
    assert_eq!(info.fail_count, 0);
}

#[test]
fn test_fail_retry_flow() {
    let state = StateManager::new(":memory:").expect("create");
    let bvid = "BV1FAIL001";

    state.insert_video(bvid, "fail", "游戏", "单机", 1, 2, 60, 100, "1", "20260608").expect("insert");

    // Fail 3 times
    for i in 0..3 {
        let count = state.increment_fail(bvid, "timeout").expect("fail");
        assert_eq!(count, i + 1);
    }

    // With max_fail=3 (condition: fail_count < 3), fail_count=3 is EXCLUDED
    let excluded = state.get_failed_bvids(3).expect("get failed with max 3");
    assert!(!excluded.contains(&bvid.to_string()), "fail_count=3 excluded by max_fail=3");

    // With max_fail=4, fail_count=3 IS included
    let included = state.get_failed_bvids(4).expect("get failed with max 4");
    assert!(included.contains(&bvid.to_string()), "fail_count=3 included by max_fail=4");

    // Reset and check
    state.reset_fail(bvid).expect("reset");
    let info = state.get_video_info(bvid).expect("info").expect("has info");
    assert_eq!(info.fail_count, 0);
}

#[test]
fn test_snapshot_tracking() {
    let state = StateManager::new(":memory:").expect("create");

    state.create_snapshot("20260608_0000").expect("create snap");
    state.complete_snapshot("20260608_0000", 1400, 50000, 100000, 49, "ok").expect("complete");

    let last = state.get_last_snapshot().expect("get last");
    assert_eq!(last, Some("20260608_0000".to_string()));

    assert!(state.has_full_crawl_today("20260608").expect("has crawl today"));
    assert!(!state.has_full_crawl_today("20260609").expect("no crawl tomorrow"));
}

#[test]
fn test_comments_and_danmaku_dedup() {
    let state = StateManager::new(":memory:").expect("create");
    let bvid = "BV1TEST002";

    state.insert_video(bvid, "test", "游戏", "单机", 1, 2, 60, 100, "1", "20260608").expect("insert");

    // Insert comments
    let comment_ids: Vec<(i64, &str)> = vec![(1, bvid), (2, bvid), (3, bvid)];
    state.insert_comments_batch(&comment_ids).expect("insert comments");

    assert!(state.is_comment_collected(1).expect("check 1"));
    assert!(!state.is_comment_collected(999).expect("check 999"));

    let ids = state.get_collected_comment_ids(bvid).expect("get ids");
    assert_eq!(ids.len(), 3);

    // Insert danmaku
    let danmaku_ids: Vec<(i64, &str)> = vec![(10, bvid), (20, bvid)];
    state.insert_danmaku_batch(&danmaku_ids).expect("insert danmaku");

    assert!(state.is_danmaku_collected(10).expect("check d10"));
    let dids = state.get_collected_danmaku_ids(bvid).expect("get dids");
    assert_eq!(dids.len(), 2);
}

#[test]
fn test_up_tracking() {
    let state = StateManager::new(":memory:").expect("create");

    assert!(!state.is_up_collected("123").expect("not yet"));

    state.mark_up_collected("123").expect("mark");
    assert!(state.is_up_collected("123").expect("now yes"));

    // Record failures
    for i in 0..3 {
        state.record_up_failure("456", "timeout").expect("fail");
    }
    assert!(state.should_skip_up("456").expect("skip after 3 fails"));

    // Fresh UP should not be skipped
    assert!(!state.should_skip_up("789").expect("fresh ok"));
}

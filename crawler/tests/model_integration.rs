use bilibili_crawler::models::{Comment, Danmaku, UpInfo, VideoDetail};
use serde_json;

#[test]
fn test_video_detail_26_fields() {
    let vd = VideoDetail {
        bvid: "BV1TEST001".into(),
        aid: 123456,
        cid: 789012,
        title: "测试标题".into(),
        description: "描述".into(),
        tags: "标签1,标签2".into(),
        category: "动画".into(),
        sub_category: "综合".into(),
        duration: 120,
        publish_time: 1700000000,
        up_id: "999".into(),
        up_name: "测试UP".into(),
        up_avatar: "https://example.com/face.jpg".into(),
        url: "https://www.bilibili.com/video/BV1TEST001".into(),
        view_count: 10000,
        like_count: 500,
        coin_count: 100,
        favorite_count: 200,
        share_count: 50,
        reply_count: 30,
        danmaku_count: 80,
        rank_source: "ranking".into(),
        rank_name: "动画榜".into(),
        rank_position: 1,
        snapshot_id: "20260608_0000".into(),
        collect_time: 1700000001000,
    };

    // JSON roundtrip
    let json = serde_json::to_string(&vd).expect("serialize");
    let parsed: VideoDetail = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.bvid, "BV1TEST001");
    assert_eq!(parsed.category, "动画");
    assert_eq!(parsed.rank_position, 1);
    assert_eq!(parsed.snapshot_id, "20260608_0000");

    // Verify all 26 fields are in JSON (count top-level keys)
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let obj = value.as_object().expect("should be object");
    assert_eq!(obj.len(), 26, "VideoDetail must have exactly 26 fields");
}

#[test]
fn test_comment_serialization() {
    let c = Comment {
        comment_id: 1,
        bvid: "BV1TEST001".into(),
        aid: 123,
        user_id: 456,
        user_name: "用户A".into(),
        content: "评论内容".into(),
        like_count: 10,
        reply_count: 2,
        comment_time: 1700000000,
    };

    let json = serde_json::to_string(&c).expect("serialize");
    let parsed: Comment = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.comment_id, 1);
    assert_eq!(parsed.content, "评论内容");
}

#[test]
fn test_danmaku_serialization() {
    let d = Danmaku {
        danmaku_id: 100,
        bvid: "BV1TEST001".into(),
        cid: 789,
        content: "弹幕测试".into(),
        video_time: 30.5,
        mode: 1,
        fontsize: 25,
        color: 16777215,
        mid_hash: "abc123".into(),
        send_time: 1700000000,
    };

    let json = serde_json::to_string(&d).expect("serialize");
    let parsed: Danmaku = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.danmaku_id, 100);
    assert_eq!(parsed.video_time, 30.5);
    assert_eq!(parsed.content, "弹幕测试");
}

#[test]
fn test_up_info_serialization() {
    let up = UpInfo {
        up_id: "999".into(),
        up_name: "测试UP".into(),
        up_avatar: "https://example.com/face.jpg".into(),
        follower_count: 50000,
        video_count: 120,
        sign: "个人简介".into(),
        official_verify: "".into(),
    };

    let json = serde_json::to_string(&up).expect("serialize");
    let parsed: UpInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.up_id, "999");
    assert_eq!(parsed.follower_count, 50000);
}

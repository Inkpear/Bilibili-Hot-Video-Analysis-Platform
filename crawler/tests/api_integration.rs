//! Integration tests that make real Bilibili API requests.
//!
//! Requires a `.env` file in the crawler directory with:
//!   SESSDATA=...
//!   BILI_JCT=...
//!   DEDE_USER_ID=...
//!
//! Run with: cargo test --test api_integration -- --include-ignored --test-threads=1

use std::collections::HashSet;
use std::time::Duration;

use bilibili_crawler::api::{comment, danmaku, ranking, up, video};
use bilibili_crawler::fingerprint::DeviceFingerprint;
use bilibili_crawler::wbi::WbiSigner;

const BASE_URL: &str = "https://api.bilibili.com";

struct TestEnv {
    client: reqwest::Client,
    cookie: String,
    wbi: WbiSigner,
}

async fn setup() -> Option<TestEnv> {
    let _ = dotenv::dotenv();

    let sessdata = std::env::var("SESSDATA").ok()?;
    let bili_jct = std::env::var("BILI_JCT").ok()?;
    let dede_user_id = std::env::var("DEDE_USER_ID").ok()?;

    if sessdata.is_empty() || bili_jct.is_empty() || dede_user_id.is_empty() {
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build client");

    let fingerprint = DeviceFingerprint::create(&client, None)
        .await
        .expect("create fingerprint");

    let cookie = fingerprint.cookie_string(&sessdata, &bili_jct, &dede_user_id);
    let wbi = WbiSigner::new(cookie.clone(), BASE_URL.to_string(), client.clone());

    Some(TestEnv {
        client,
        cookie,
        wbi,
    })
}

fn delay_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

// ---------------------------------------------------------------------------
// SPI / Fingerprint
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_spi_fingerprint() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let fp = DeviceFingerprint::create(&client, None)
        .await
        .expect("SPI should return buvid3/buvid4");

    assert!(!fp.buvid3.is_empty(), "buvid3 should not be empty");
    assert!(!fp.buvid4.is_empty(), "buvid4 should not be empty");
    assert!(
        fp.buvid3.len() > 10,
        "buvid3 looks too short: {}",
        fp.buvid3
    );
    assert!(!fp.uuid.is_empty(), "uuid should be generated");
    assert!(!fp.user_agent.is_empty(), "user_agent should be set");

    println!("buvid3: {}", fp.buvid3);
    println!("buvid4: {}", fp.buvid4);
    println!("ua: {}", fp.user_agent);
}

#[tokio::test]
#[ignore]
async fn test_spi_different_ua_different_buvid() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let fp1 = DeviceFingerprint::create(
        &client,
        Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/131.0.0.0"),
    )
    .await
    .expect("first SPI call");

    delay_ms(1000);

    let fp2 = DeviceFingerprint::create(
        &client,
        Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Safari/605.1.15"),
    )
    .await
    .expect("second SPI call");

    assert_ne!(
        fp1.buvid3, fp2.buvid3,
        "different UAs should yield different buvid3"
    );
}

// ---------------------------------------------------------------------------
// Rankings
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_fetch_all_rankings() {
    let env = setup()
        .await
        .expect("set SESSDATA/BILI_JCT/DEDE_USER_ID in .env");

    let ranked = ranking::fetch_all_rankings(
        &env.client,
        &env.cookie,
        BASE_URL,
        &env.wbi,
        1500, // delay between ranking requests
    )
    .await
    .expect("fetch rankings should succeed");

    assert!(
        !ranked.is_empty(),
        "should return at least some ranked videos"
    );

    let first = &ranked[0];
    assert!(!first.bvid.is_empty(), "bvid should not be empty");
    assert!(!first.title.is_empty(), "title should not be empty");
    assert!(first.aid > 0, "aid should be positive");
    assert!(first.cid > 0, "cid should be positive");
    assert!(!first.category.is_empty(), "category should not be empty");
    assert!(first.view_count >= 0, "view_count should be non-negative");

    println!("fetched {} ranked videos", ranked.len());
    println!(
        "sample: {} - {} ({})",
        first.bvid, first.title, first.category
    );
}

// ---------------------------------------------------------------------------
// Video Detail
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_fetch_video_detail() {
    let env = setup()
        .await
        .expect("set SESSDATA/BILI_JCT/DEDE_USER_ID in .env");

    let ranked = ranking::fetch_all_rankings(&env.client, &env.cookie, BASE_URL, &env.wbi, 1500)
        .await
        .expect("fetch rankings");

    let bvid = &ranked[0].bvid;
    delay_ms(2000);

    let detail = video::fetch_video_detail(&env.client, &env.cookie, BASE_URL, bvid)
        .await
        .expect("fetch video detail should succeed");

    assert_eq!(detail.bvid, *bvid);
    assert!(detail.aid > 0, "aid should be positive");
    assert!(detail.cid > 0, "cid should be positive");
    assert!(!detail.title.is_empty(), "title should not be empty");

    println!(
        "detail: {} - aid={}, cid={}",
        detail.bvid, detail.aid, detail.cid
    );
    println!("  tags: {}", detail.tags);
    let desc_preview: String = detail.description.chars().take(100).collect();
    println!("  desc: {}", desc_preview);
}

// ---------------------------------------------------------------------------
// UP Info
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_fetch_up_info() {
    let env = setup()
        .await
        .expect("set SESSDATA/BILI_JCT/DEDE_USER_ID in .env");

    let ranked = ranking::fetch_all_rankings(&env.client, &env.cookie, BASE_URL, &env.wbi, 1500)
        .await
        .expect("fetch rankings");

    let up_id = &ranked[0].up_id;
    delay_ms(2000);

    let info = up::fetch_up_info(&env.client, &env.cookie, BASE_URL, &env.wbi, up_id)
        .await
        .expect("fetch up info should succeed");

    assert_eq!(info.up_id, *up_id);

    if info.up_name.is_empty() {
        println!(
            "up: {} - name empty (API may have returned non-zero code)",
            info.up_id
        );
    } else {
        println!(
            "up: {} - {} (followers: {}, videos: {})",
            info.up_id, info.up_name, info.follower_count, info.video_count
        );
    }
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_fetch_comments() {
    let env = setup()
        .await
        .expect("set SESSDATA/BILI_JCT/DEDE_USER_ID in .env");

    let ranked = ranking::fetch_all_rankings(&env.client, &env.cookie, BASE_URL, &env.wbi, 1500)
        .await
        .expect("fetch rankings");

    let rv = &ranked[0];
    let existing = HashSet::new();
    delay_ms(2000);

    let (comments, _success) = comment::collect_comments(
        &env.client,
        &env.cookie,
        BASE_URL,
        &rv.bvid,
        rv.aid,
        20,
        &existing,
    )
    .await
    .expect("fetch comments should succeed");

    println!("got {} comments for {}", comments.len(), rv.bvid);
    if !comments.is_empty() {
        let c = &comments[0];
        let preview: String = c.content.chars().take(80).collect();
        println!("  sample: [{}] {}", c.user_name, preview);
    }
}

// ---------------------------------------------------------------------------
// Danmaku
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_fetch_danmaku() {
    let env = setup()
        .await
        .expect("set SESSDATA/BILI_JCT/DEDE_USER_ID in .env");

    let ranked = ranking::fetch_all_rankings(&env.client, &env.cookie, BASE_URL, &env.wbi, 1500)
        .await
        .expect("fetch rankings");

    let rv = &ranked[0];
    let existing = HashSet::new();
    delay_ms(2000);

    let (danmaku_list, _last_seg) = danmaku::collect_danmaku(
        &env.client,
        &env.cookie,
        &rv.bvid,
        rv.cid,
        rv.danmaku_count,
        50,
        0,
        &existing,
    )
    .await
    .expect("fetch danmaku should succeed");

    println!("got {} danmaku for {}", danmaku_list.len(), rv.bvid);
    if !danmaku_list.is_empty() {
        let d = &danmaku_list[0];
        let preview: String = d.content.chars().take(80).collect();
        println!("  sample: [{}] {}", d.mode, preview);
    }
}

// ---------------------------------------------------------------------------
// Worker (full integration)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_worker_create() {
    let _ = dotenv::dotenv();

    let sessdata = std::env::var("SESSDATA").unwrap_or_default();
    let bili_jct = std::env::var("BILI_JCT").unwrap_or_default();
    let dede_user_id = std::env::var("DEDE_USER_ID").unwrap_or_default();

    if sessdata.is_empty() {
        eprintln!("SESSDATA not set, skipping");
        return;
    }

    use bilibili_crawler::config::AccountConfig;

    let config = AccountConfig {
        sessdata,
        bili_jct,
        dede_user_id,
        proxy: None,
        user_agent: None,
    };

    let worker = bilibili_crawler::worker::Worker::create(0, &config, BASE_URL, 500, 2000)
        .await
        .expect("worker creation should succeed");

    assert!(!worker.fingerprint.buvid3.is_empty());
    assert!(!worker.cookie().is_empty());

    println!(
        "worker created: buvid3={}, ua={}",
        worker.fingerprint.buvid3, worker.fingerprint.user_agent
    );
}

// ---------------------------------------------------------------------------
// Cookie string
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn test_cookie_string_contains_all_fields() {
    let _ = dotenv::dotenv();

    let sessdata = std::env::var("SESSDATA").unwrap_or_default();
    let bili_jct = std::env::var("BILI_JCT").unwrap_or_default();
    let dede_user_id = std::env::var("DEDE_USER_ID").unwrap_or_default();

    if sessdata.is_empty() {
        eprintln!("SESSDATA not set, skipping");
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let fp = DeviceFingerprint::create(&client, None).await.expect("SPI");

    let cookie = fp.cookie_string(&sessdata, &bili_jct, &dede_user_id);

    assert!(cookie.contains("buvid3="));
    assert!(cookie.contains("buvid4="));
    assert!(cookie.contains("b_nut="));
    assert!(cookie.contains("_uuid="));
    assert!(cookie.contains(&format!("SESSDATA={}", sessdata)));
    assert!(cookie.contains(&format!("bili_jct={}", bili_jct)));
    assert!(cookie.contains(&format!("DedeUserID={}", dede_user_id)));

    println!("cookie: {}", cookie);
}

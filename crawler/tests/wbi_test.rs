//! WBI signing correctness test — runs against both real accounts from config.toml.
//!
//! Run: cargo test --test wbi_test -- --include-ignored --test-threads=1

use std::collections::HashMap;
use std::time::Duration;

use bilibili_crawler::fingerprint::DeviceFingerprint;
use bilibili_crawler::wbi::WbiSigner;
use serde_json::Value;

const BASE_URL: &str = "https://api.bilibili.com";

struct Account {
    name: &'static str,
    sessdata: &'static str,
    bili_jct: &'static str,
    dede_user_id: &'static str,
}

const ACCOUNTS: [Account; 2] = [
    Account {
        name: "account1",
        sessdata: "b5461955%2C1796731989%2C83bf5%2A62",
        bili_jct: "d2d2bb01b3e280a808a00eb93111861e",
        dede_user_id: "625317796",
    },
    Account {
        name: "account2",
        sessdata: "fa177cf1%2C1795738942%2Cbb054%2A52CjCGFxOC4A900rSqVG1-Tqub9A41nag9ZfL9MBWnhBUGu-DfXOryqHPP2b_3qQy1U_MSVjd4V0ZNRjdfMXZ0TWVBRzVMNHRfZWNNbHVhbWdYanpLM3hZaGQ2STFrYUZpWHZmejNaUDNpc1F5clVtcVl5cDhkVkJrTXFCRU1WNENzRE9HZHNaOGxBIIEC",
        bili_jct: "c85656f4a514d1cb378d64d44bff7359",
        dede_user_id: "3546969739888768",
    },
];

/// Setup a shared client + cookie + WBI for one account.
/// Only does ONE SPI call per invocation.
async fn setup(account: &Account) -> (reqwest::Client, String, WbiSigner, DeviceFingerprint) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build client");

    let fp = DeviceFingerprint::create(&client, None)
        .await
        .expect("SPI fingerprint");

    let cookie = fp.cookie_string(account.sessdata, account.bili_jct, account.dede_user_id);
    let wbi = WbiSigner::new(cookie.clone(), BASE_URL.to_string(), client.clone());

    (client, cookie, wbi, fp)
}

// ---------------------------------------------------------------------------
// Account 1: all WBI endpoints in sequence (one setup, one WBI key fetch)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn account1_all_wbi_endpoints() {
    let (client, cookie, wbi, fp) = setup(&ACCOUNTS[0]).await;

    // Debug: print cookie contents
    println!(
        "[a1] cookie keys: {}",
        cookie
            .split("; ")
            .map(|p| p.split('=').next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("[a1] cookie length: {}", cookie.len());
    println!("[a1] UA: {}", fp.user_agent);

    // 0. Check SESSDATA validity via nav endpoint
    {
        let resp = fp
            .apply_headers(client.get(&format!("{}/x/web-interface/nav", BASE_URL)))
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let is_login = json
            .pointer("/data/isLogin")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let uname = json
            .pointer("/data/uname")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!(
            "[a1] nav code={}, isLogin={}, uname={}",
            code, is_login, uname
        );
        assert!(is_login, "[a1] SESSDATA appears expired — not logged in");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 1. WBI key fetch (implicit in sign_params)
    {
        let mut params = HashMap::new();
        params.insert("mid".to_string(), "1".to_string());
        let result = wbi.sign_params(&mut params).await;
        assert!(result.is_ok(), "[a1] WBI sign_params failed: {:?}", result);
        assert!(params.contains_key("wts"));
        assert!(params.contains_key("w_rid"));
        println!("[a1] WBI key fetch OK, w_rid={}", params["w_rid"]);
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 2. acc/info (Referer: www.bilibili.com)
    {
        let mut params = HashMap::new();
        params.insert("mid".to_string(), "2".to_string());
        wbi.sign_params(&mut params).await.expect("sign");

        let url = format!("{}/x/space/wbi/acc/info", BASE_URL);
        println!("[a1] acc/info URL: {}", url);
        println!("[a1] acc/info params: {:?}", params);

        let resp = fp
            .apply_headers(client.get(&url))
            .query(&params)
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let preview: String = raw.chars().take(300).collect();
        println!("[a1] acc/info raw response: {}", preview);

        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        println!("[a1] acc/info code={}, message={}", code, msg);

        if code == -352 {
            println!("[a1] acc/info -352 data: {:?}", json.get("data"));
        }
        assert_eq!(code, 0, "[a1] acc/info failed: code={}, msg={}", code, msg);

        let name = json
            .pointer("/data/name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("[a1] acc/info name={}", name);
        assert!(!name.is_empty(), "[a1] UP name is empty");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 3. arc/search (Referer: space.bilibili.com/{mid}/video)
    {
        let mut params = HashMap::new();
        params.insert("mid".to_string(), "2".to_string());
        params.insert("pn".to_string(), "1".to_string());
        params.insert("ps".to_string(), "1".to_string());
        wbi.sign_params(&mut params).await.expect("sign");

        let referer = "https://space.bilibili.com/2/video";
        let resp = fp
            .apply_headers(client.get(&format!("{}/x/space/wbi/arc/search", BASE_URL)))
            .query(&params)
            .header("Cookie", &cookie)
            .header("Referer", referer)
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        println!("[a1] arc/search code={}, message={}", code, msg);

        if code == -352 {
            println!("[a1] arc/search -352 data: {:?}", json.get("data"));
        }
        assert_eq!(
            code, 0,
            "[a1] arc/search failed: code={}, msg={}",
            code, msg
        );

        let count = json
            .pointer("/data/page/count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("[a1] arc/search video_count={}", count);
        assert!(count > 0, "[a1] video count should be positive");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 4. ranking/v2 (Referer: www.bilibili.com)
    {
        let mut params = HashMap::new();
        params.insert("rid".to_string(), "0".to_string());
        params.insert("type".to_string(), "all".to_string());
        wbi.sign_params(&mut params).await.expect("sign");

        let resp = fp
            .apply_headers(client.get(&format!("{}/x/web-interface/ranking/v2", BASE_URL)))
            .query(&params)
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        println!("[a1] ranking/v2 code={}, message={}", code, msg);

        if code == -352 {
            println!("[a1] ranking -352 data: {:?}", json.get("data"));
        }
        assert_eq!(code, 0, "[a1] ranking failed: code={}, msg={}", code, msg);

        let list = json.pointer("/data/list").and_then(|v| v.as_array());
        assert!(list.is_some(), "[a1] ranking list missing");
        println!("[a1] ranking list len={}", list.unwrap().len());
    }

    println!("[a1] ALL WBI ENDPOINTS PASSED");
}

// ---------------------------------------------------------------------------
// Account 2: same sequence
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn account2_all_wbi_endpoints() {
    // Delay before starting account2 to avoid cross-account rate limiting
    tokio::time::sleep(Duration::from_secs(3)).await;

    let (client, cookie, wbi, fp) = setup(&ACCOUNTS[1]).await;

    // 0. Check SESSDATA validity
    {
        let resp = fp
            .apply_headers(client.get(&format!("{}/x/web-interface/nav", BASE_URL)))
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let is_login = json
            .pointer("/data/isLogin")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let uname = json
            .pointer("/data/uname")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!(
            "[a2] nav code={}, isLogin={}, uname={}",
            code, is_login, uname
        );
        assert!(is_login, "[a2] SESSDATA appears expired — not logged in");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 1. WBI key fetch
    {
        let mut params = HashMap::new();
        params.insert("mid".to_string(), "1".to_string());
        let result = wbi.sign_params(&mut params).await;
        assert!(result.is_ok(), "[a2] WBI sign_params failed: {:?}", result);
        println!("[a2] WBI key fetch OK, w_rid={}", params["w_rid"]);
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 2. acc/info
    {
        let mut params = HashMap::new();
        params.insert("mid".to_string(), "2".to_string());
        wbi.sign_params(&mut params).await.expect("sign");

        let resp = fp
            .apply_headers(client.get(&format!("{}/x/space/wbi/acc/info", BASE_URL)))
            .query(&params)
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        println!("[a2] acc/info code={}, message={}", code, msg);

        if code == -352 {
            println!("[a2] acc/info -352 data: {:?}", json.get("data"));
        }
        assert_eq!(code, 0, "[a2] acc/info failed: code={}, msg={}", code, msg);

        let name = json
            .pointer("/data/name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("[a2] acc/info name={}", name);
        assert!(!name.is_empty(), "[a2] UP name is empty");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 3. arc/search
    {
        let mut params = HashMap::new();
        params.insert("mid".to_string(), "2".to_string());
        params.insert("pn".to_string(), "1".to_string());
        params.insert("ps".to_string(), "1".to_string());
        wbi.sign_params(&mut params).await.expect("sign");

        let referer = "https://space.bilibili.com/2/video";
        let resp = fp
            .apply_headers(client.get(&format!("{}/x/space/wbi/arc/search", BASE_URL)))
            .query(&params)
            .header("Cookie", &cookie)
            .header("Referer", referer)
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        println!("[a2] arc/search code={}, message={}", code, msg);

        if code == -352 {
            println!("[a2] arc/search -352 data: {:?}", json.get("data"));
        }
        assert_eq!(
            code, 0,
            "[a2] arc/search failed: code={}, msg={}",
            code, msg
        );

        let count = json
            .pointer("/data/page/count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("[a2] arc/search video_count={}", count);
        assert!(count > 0, "[a2] video count should be positive");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // 4. ranking/v2
    {
        let mut params = HashMap::new();
        params.insert("rid".to_string(), "0".to_string());
        params.insert("type".to_string(), "all".to_string());
        wbi.sign_params(&mut params).await.expect("sign");

        let resp = fp
            .apply_headers(client.get(&format!("{}/x/web-interface/ranking/v2", BASE_URL)))
            .query(&params)
            .header("Cookie", &cookie)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .expect("HTTP");

        let raw = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).expect("JSON");
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-999);
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        println!("[a2] ranking/v2 code={}, message={}", code, msg);

        if code == -352 {
            println!("[a2] ranking -352 data: {:?}", json.get("data"));
        }
        assert_eq!(code, 0, "[a2] ranking failed: code={}, msg={}", code, msg);

        let list = json.pointer("/data/list").and_then(|v| v.as_array());
        assert!(list.is_some(), "[a2] ranking list missing");
        println!("[a2] ranking list len={}", list.unwrap().len());
    }

    println!("[a2] ALL WBI ENDPOINTS PASSED");
}

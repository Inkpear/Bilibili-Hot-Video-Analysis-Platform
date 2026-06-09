// examples/api_test.rs — Test Bilibili API with buvid3 + WBI signing
//
// Usage: BILI_COOKIE="SESSDATA=..." cargo run --example api_test

use std::collections::HashMap;
use std::env;
use md5::{Md5, Digest};

const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49,
    33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61,
    26, 17, 0, 1, 60, 51, 30, 4, 22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36,
    20, 34, 44, 52,
];

fn get_mixin_key(orig: &str) -> String {
    MIXIN_KEY_ENC_TAB
        .iter()
        .filter_map(|&i| orig.chars().nth(i))
        .take(32)
        .collect()
}

fn extract_key_from_url(url: &str) -> String {
    url.rsplit('/').next().unwrap().split('.').next().unwrap().to_string()
}

fn sign_params(params: &mut HashMap<String, String>, mixin_key: &str) {
    let wts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    params.insert("wts".to_string(), wts);

    let mut sorted: Vec<(&String, &String)> = params.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let chars_to_strip = ['!', '\'', '(', ')', '*'];
    let query: String = sorted
        .iter()
        .map(|(k, v)| {
            let cleaned: String = v.chars().filter(|c| !chars_to_strip.contains(c)).collect();
            format!("{}={}", urlencoding(k), urlencoding(&cleaned))
        })
        .collect::<Vec<_>>()
        .join("&");

    let hash = Md5::digest(format!("{}{}", query, mixin_key));
    let w_rid = format!("{:x}", hash);
    params.insert("w_rid".to_string(), w_rid);
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cookie = env::var("BILI_COOKIE").expect("BILI_COOKIE not set");
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    println!("=== Step 1: Fetch buvid3 ===");
    let spi: serde_json::Value = client
        .get("https://api.bilibili.com/x/frontend/finger/spi")
        .header("Cookie", &cookie)
        .send().await?.json().await?;
    let b3 = spi["data"]["b_3"].as_str().unwrap_or("");
    println!("buvid3 = {}", &b3[..b3.len().min(30)]);

    let augmented_cookie = format!("buvid3={}; {}", b3, cookie);
    println!("cookie_len = {}", augmented_cookie.len());

    println!("\n=== Step 2: Fetch WBI keys ===");
    let nav: serde_json::Value = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .header("Cookie", &augmented_cookie)
        .send().await?.json().await?;
    let img_url = nav["data"]["wbi_img"]["img_url"].as_str().unwrap();
    let sub_url = nav["data"]["wbi_img"]["sub_url"].as_str().unwrap();
    let img_key = extract_key_from_url(img_url);
    let sub_key = extract_key_from_url(sub_url);
    let mixin_key = get_mixin_key(&format!("{}{}", img_key, sub_key));
    println!("img_key={}, sub_key={}, mixin={}", img_key, sub_key, mixin_key);

    println!("\n=== Step 3: Call ranking API (no WBI) ===");
    let resp1 = client
        .get("https://api.bilibili.com/x/web-interface/ranking/v2?rid=1005&type=all")
        .header("Cookie", &augmented_cookie)
        .header("Referer", "https://www.bilibili.com")
        .send().await?;
    let text1 = resp1.text().await?;
    let code1: i32 = serde_json::from_str::<serde_json::Value>(&text1)
        .map(|v| v["code"].as_i64().unwrap_or(-999) as i32)
        .unwrap_or(-999);
    println!("Without WBI: code={}", code1);

    println!("\n=== Step 4: Call ranking API (WITH WBI) ===");
    let mut params: HashMap<String, String> = HashMap::new();
    params.insert("rid".to_string(), "1005".to_string());
    params.insert("type".to_string(), "all".to_string());
    sign_params(&mut params, &mixin_key);

    let resp2 = client
        .get("https://api.bilibili.com/x/web-interface/ranking/v2")
        .query(&params)
        .header("Cookie", &augmented_cookie)
        .header("Referer", "https://www.bilibili.com")
        .send().await?;
    let text2 = resp2.text().await?;
    let v2: serde_json::Value = serde_json::from_str(&text2)?;
    let code2 = v2["code"].as_i64().unwrap_or(-999) as i32;
    let count = v2["data"]["list"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    println!("With WBI: code={}, videos={}", code2, count);

    if code2 == 0 {
        println!("\n✅ SUCCESS! Ranking API works with buvid3 + WBI!");
    } else {
        println!("\n❌ FAILED: code={}", code2);
    }

    Ok(())
}

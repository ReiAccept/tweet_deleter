use anyhow::Context;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;
use std::time::Duration;


// `{"tweet": {"tweet_id": "..."}}`
#[derive(Deserialize, Debug)]
struct TweetEntry {
    tweet: Tweet,
}

#[derive(Deserialize, Debug)]
struct Tweet {
    tweet_id: String,
}

#[derive(Serialize, Debug)]
struct DeletePayload<'a> {
    variables: Variables<'a>,
    #[serde(rename = "queryId")]
    query_id: &'a str,
}

#[derive(Serialize, Debug)]
struct Variables<'a> {
    tweet_id: &'a str,
    #[serde(rename = "dark_request")]
    dark_request: bool,
}

fn get_tweet_ids(json_data: &str) -> anyhow::Result<Vec<String>> {
    let entries: Vec<TweetEntry> = serde_json::from_str(json_data)
        .context("Read tweet-headers.js Faild")?;

    let mut ids: Vec<String> = entries.into_iter().map(|entry| entry.tweet.tweet_id).collect();

    // 反转顺序，从后往前删
    ids.reverse();

    Ok(ids)
}

fn delete_tweet(client: &Client, headers: &HeaderMap, tweet_id: &str) -> anyhow::Result<()> {
    println!("[*] Delete Tweet ID: {}", tweet_id);

    let delete_url = "https://twitter.com/i/api/graphql/VaenaVgh5q5ih7kvyVjgtg/DeleteTweet";

    let payload = DeletePayload {
        variables: Variables {
            tweet_id,
            dark_request: false,
        },
        query_id: "VaenaVgh5q5ih7kvyVjgtg",
    };

    let response = client
        .post(delete_url)
        .headers(headers.clone())
        .json(&payload)
        .send()
        .context(format!("Faild (tweet_id: {})", tweet_id))?;

    println!(
        "[*] Status: {} {}",
        response.status().as_str(),
        response.status().canonical_reason().unwrap_or("")
    );

    let response_text = response
        .text()
        .context("Read resp fail")?;
    println!("[*] Response: {}", response_text);

    Ok(())
}


fn main() -> anyhow::Result<()> {

    let raw_content = fs::read_to_string("tweet-headers.js")
        .context("Read tweet-headers.js faild")?;
    

    let start_index = raw_content.find('[').context("Can't find json in  tweet-headers.js ('[')")?;
    let json_slice = &raw_content[start_index..];
    
    let ids = get_tweet_ids(json_slice)?;
    println!("Found {} Tweets", ids.len());


    let mut headers = HeaderMap::new();
    let headers_content = fs::read_to_string("headers.txt")
        .context("Read headers.txt faild")?;

    for line in headers_content.lines() {

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if let (Ok(header_name), Ok(header_value)) = (HeaderName::from_str(key), HeaderValue::from_str(value)) {
                headers.insert(header_name, header_value);
            }
        }
    }

    headers.insert("content-type", HeaderValue::from_static("application/json"));


    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let total = ids.len();

    for (index, id) in ids.iter().enumerate() {
        println!("--- At: {}/{} ---", index + 1, total);
        if let Err(e) = delete_tweet(&client, &headers, id) {
            eprintln!("[!] Delete {} faild: {:?}\n", id, e);
        }
        
        std::thread::sleep(Duration::from_millis(500)); 
    }

    println!("\n Finish");

    Ok(())
}
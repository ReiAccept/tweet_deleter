use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;
use std::time::Duration;

const DELETE_URL: &str = "https://twitter.com/i/api/graphql/VaenaVgh5q5ih7kvyVjgtg/DeleteTweet";
const QUERY_ID: &str = "VaenaVgh5q5ih7kvyVjgtg";
const TWEET_DATA_FILE: &str = "tweet-headers.js";
const HEADERS_FILE: &str = "headers.txt";

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

fn get_tweet_ids_from_file(path: &str) -> Result<Vec<String>> {
    let raw_content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read tweet data file '{}'", path))?;

    let start_index = raw_content.find('[')
        .with_context(|| format!("Could not find start of JSON array '[' in '{}'", path))?;
    let json_slice = &raw_content[start_index..];

    let entries: Vec<TweetEntry> = serde_json::from_str(json_slice)
        .context("Failed to parse tweet JSON data")?;

    let mut ids: Vec<String> = entries.into_iter().map(|entry| entry.tweet.tweet_id).collect();

    ids.reverse();

    Ok(ids)
}

fn load_headers(path: &str) -> Result<HeaderMap> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read headers file '{}'", path))?;

    let mut headers = HeaderMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if let (Ok(header_name), Ok(header_value)) =
                (HeaderName::from_str(key), HeaderValue::from_str(value))
            {
                headers.insert(header_name, header_value);
            } else {
                eprintln!("[!] Warning: Ignoring invalid header: {}", line);
            }
        }
    }
    
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    Ok(headers)
}

fn delete_tweet(client: &Client, headers: &HeaderMap, tweet_id: &str) -> Result<()> {
    println!("[*] Deleting Tweet ID: {}", tweet_id);

    let payload = DeletePayload {
        variables: Variables {
            tweet_id,
            dark_request: false,
        },
        query_id: QUERY_ID,
    };

    let response = client
        .post(DELETE_URL)
        .headers(headers.clone())
        .json(&payload)
        .send()
        .with_context(|| format!("Request to delete tweet {} failed", tweet_id))?;

    let status = response.status();
    let response_text = response.text().context("Failed to read response body")?;

    println!("[*] Status: {}", status);
    println!("[*] Response: {}", response_text.trim());

    if !status.is_success() {
        return Err(anyhow!("API returned non-success status: {}", status));
    }

    Ok(())
}

fn main() -> Result<()> {
    let ids = get_tweet_ids_from_file(TWEET_DATA_FILE)?;
    let total = ids.len();
    println!("Found {} tweets to delete.", total);

    if total == 0 {
        println!("No tweets to delete.");
        return Ok(());
    }

    let headers = load_headers(HEADERS_FILE)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    for (index, id) in ids.iter().enumerate() {
        println!("--- Progress: {}/{} ---", index + 1, total);
        if let Err(e) = delete_tweet(&client, &headers, id) {
            eprintln!("[!] Failed to delete tweet {}: {:?}\n", id, e);
        }
        
        std::thread::sleep(Duration::from_millis(500));
    }

    println!("\nAll operations completed.");
    Ok(())
}

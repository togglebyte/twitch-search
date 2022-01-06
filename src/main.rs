use std::env;
use std::process::exit;

use chrono::prelude::*;
use serde_json::Value;

const ROOT_URL: &'static str = "https://api.twitch.tv/helix/streams?first=100;game_id=1469308723";

macro_rules! to_str {
    ($val: expr, $key: expr) => {
        $val.get($key).unwrap().as_str().unwrap().to_string()
    };
}

macro_rules! to_num {
    ($val: expr, $key: expr) => {
        $val.get($key).unwrap().as_i64().unwrap()
    };
}

fn to_instant(ds: &str) -> String {
    match ds.parse::<DateTime<Utc>>() {
        Ok(val) => {
            let dur = Utc::now() - val;
            format!("{:02}:{:02}", dur.num_hours(), dur.num_minutes() % 60)
        }
        Err(_e) => "".to_string(),
    }
}

#[derive(Debug)]
struct Entry {
    lang: String,
    display_name: String,
    title: String,
    game_id: String,
    viewer_count: i64,
    live_duration: String,
}

fn filter(entry: &Entry, term: &str, ignored_names: &[&str]) -> bool {
    let display_name: &str = &entry.display_name.to_lowercase();
    //
    if ignored_names.contains(&display_name) {
        return false;
    }

    if entry.title.to_lowercase().contains(term) {
        true
    } else {
        false
    }
}

fn print(entry: Entry) {
    print!("{} | ", entry.lang);
    print!("https://twitch.tv/{:<14} | ", entry.display_name);
    print!("{:>4} viewers | ", entry.viewer_count);
    print!("{} | ", entry.live_duration);
    print!("{}\n", entry.title);
}

fn to_entry(value: &mut Value) -> Entry {
    let value = value.take();

    Entry {
        lang: to_str!(value, "language"),
        display_name: to_str!(value, "user_name"),
        title: to_str!(value, "title"),
        game_id: to_str!(value, "game_id"),
        viewer_count: to_num!(value, "viewer_count"),
        live_duration: to_instant(&to_str!(value, "started_at")),
    }
}

fn fetch(after: Option<String>) -> (Vec<Entry>, Option<String>) {
    let url = match after {
        Some(after) => format!("{}&after={}", ROOT_URL, after),
        None => ROOT_URL.to_string(),
    };

    let client_id = match env::var("TWITCH_CLIENT_ID") {
        Ok(cid) => cid,
        Err(_e) => {
            eprintln!("Client id missing");
            exit(1);
        }
    };

    let token = match env::var("TWITCH_TOKEN") {
        Ok(t) => t,
        Err(_e) => {
            eprintln!("OAuth token missing");
            exit(1);
        }
    };

    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Client-Id", &client_id)
        .call();

    let mut json: Value = match resp.unwrap().into_json() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("failed to serialize json: {:?}", e);
            exit(1);
        }
    };

    let pagination = json
        .get_mut("pagination")
        .take()
        .and_then(|v| v.get("cursor").take())
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let data = match json.get_mut("data") {
        Some(Value::Array(a)) => a.into_iter().map(to_entry).collect::<Vec<_>>(),
        _ => {
            exit(0);
        }
    };

    (data, pagination)
}

fn main() {
    let search_term = match std::env::args().skip(1).next() {
        Some(term) => term.to_lowercase(),
        None => "".to_string(),
    };

    println!("Searching for \"{}\"", search_term);

    let mut total = 0;
    let mut found = 0;

    let mut page = None;
    loop {
        let (entries, p) = fetch(page);
        total += entries.len();
        page = p;
        for entry in entries
            .into_iter()
            .filter(|e| filter(e, &search_term, &["kaetempest", "skarab42", "togglebit"]))
            .collect::<Vec<_>>()
        {
            print(entry);
            found += 1;
        }

        if page.is_none() {
            break;
        }
    }
    println!("Done ({}/{})", found, total);
}

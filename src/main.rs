use anyhow::{anyhow, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameData {
    pub home_score: f64,
    pub away_score: f64,
    pub home_odds: f64,
    pub away_odds: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    pub game_id: String,
    pub end_time: Option<String>,
    pub data: GameData,
}

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    pub data: Vec<Game>,
}

#[derive(Debug, Default)]
struct Bucket {
    pub total: AtomicUsize,
    pub wins: AtomicUsize,
}

fn main() -> Result<()> {
    let path = env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow!("Path missing!"))?;
    let bucket_amount: usize = env::args()
        .nth(2)
        .ok_or_else(|| anyhow!("Buckets missing!"))?
        .parse()?;
    let mut buckets = Vec::with_capacity(bucket_amount);
    buckets.resize_with(bucket_amount, Bucket::default);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let json: Payload = serde_json::from_reader(reader)?;
    json.data
        .par_iter()
        .filter(|game| game.end_time.is_some())
        .for_each(|game| {
            let home_bucket = (game.data.home_odds * bucket_amount as f64).floor() as usize;
            let away_bucket = (game.data.away_odds * bucket_amount as f64).floor() as usize;
            buckets[home_bucket].total.fetch_add(1, Ordering::Relaxed);
            buckets[away_bucket].total.fetch_add(1, Ordering::Relaxed);
            if game.data.home_score > game.data.away_score {
                buckets[home_bucket].wins.fetch_add(1, Ordering::Relaxed);
            } else if game.data.home_score < game.data.away_score {
                buckets[away_bucket].wins.fetch_add(1, Ordering::Relaxed);
            } else {
                unreachable!("tie???????? {}", game.game_id);
            }
        });
    for (i, bucket) in buckets.iter().enumerate() {
        let min = i as f64 * 100.0 / bucket_amount as f64;
        let max = (i + 1) as f64 * 100.0 / bucket_amount as f64;
        let total = bucket.total.load(Ordering::SeqCst);
        if total == 0 {
            println!("{}%-{}%: no data", min, max);
        } else {
            let wins = bucket.wins.load(Ordering::SeqCst);
            let expected = ((wins as f64) / (total as f64)) * 100.0;
            println!("{}%-{}%: {}% ({} games)", min, max, expected, total);
        }
    }
    Ok(())
}

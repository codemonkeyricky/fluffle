//! Token usage statistics: per-context recording and ASCII histogram rendering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRecord {
    cid: u64,
    total_tokens: u32,
    timestamp: u64,
}

/// Records token usage per context ID and renders a histogram at program exit.
pub struct TokenStatsRecorder {
    file_path: PathBuf,
    /// Accumulated tokens per CID in the current run.
    current: HashMap<u64, u32>,
    /// Historical per-context token counts loaded from file at startup.
    historical: Vec<u32>,
}

impl TokenStatsRecorder {
    pub fn new() -> Self {
        let file_path = Self::default_path();
        let historical = Self::load_history(&file_path);
        Self {
            file_path,
            current: HashMap::new(),
            historical,
        }
    }

    fn default_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nanocode")
            .join("token_stats.jsonl")
    }

    fn load_history(path: &PathBuf) -> Vec<u32> {
        let mut results = Vec::new();
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if let Ok(record) = serde_json::from_str::<SessionRecord>(line) {
                    if record.total_tokens > 0 {
                        results.push(record.total_tokens);
                    }
                }
            }
        }
        results
    }

    /// Accumulate tokens for a context. May be called multiple times per CID.
    pub fn add_tokens(&mut self, cid: u64, tokens: u32) {
        *self.current.entry(cid).or_insert(0) += tokens;
    }

    /// Persist current-run data to disk and print histogram to stdout.
    pub fn save_and_plot(&self) {
        self.save_to_file();
        self.render_histogram();
    }

    fn save_to_file(&self) {
        if self.current.is_empty() {
            return;
        }
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
        else {
            return;
        };
        use std::io::Write;
        for (&cid, &total_tokens) in &self.current {
            if total_tokens == 0 {
                continue;
            }
            let record = SessionRecord { cid, total_tokens, timestamp: now };
            if let Ok(line) = serde_json::to_string(&record) {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    fn render_histogram(&self) {
        let mut all_tokens: Vec<u32> = self.historical.clone();
        all_tokens.extend(self.current.values().filter(|&&t| t > 0).copied());

        if all_tokens.is_empty() {
            println!("\nNo token statistics recorded.");
            return;
        }

        let max_tokens = *all_tokens.iter().max().unwrap();
        if max_tokens == 0 {
            return;
        }

        const NUM_BINS: usize = 10;
        let bin_size = ((max_tokens as f64 / NUM_BINS as f64).ceil() as u32).max(1);

        let mut bins = vec![0usize; NUM_BINS];
        for &tokens in &all_tokens {
            let idx = ((tokens / bin_size) as usize).min(NUM_BINS - 1);
            bins[idx] += 1;
        }

        let max_count = *bins.iter().max().unwrap_or(&1);
        let max_count = max_count.max(1);
        const BAR_WIDTH: usize = 40;

        println!("\n=== Token Usage Distribution ({} contexts total) ===", all_tokens.len());
        println!();
        for (i, &count) in bins.iter().enumerate() {
            let lo = i as u32 * bin_size;
            let hi = (i as u32 + 1) * bin_size - 1;
            let filled = count * BAR_WIDTH / max_count;
            let bar = format!("{}{}", "#".repeat(filled), ".".repeat(BAR_WIDTH - filled));
            println!("{:>8}-{:<8} | {} {:>4}", lo, hi, bar, count);
        }
        println!();
        println!("x: tokens consumed per context  |  y: number of contexts");
    }
}

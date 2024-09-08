use std::collections::HashMap;

const MIN_MAX_DISTANCE: usize = 100;
const MAX_MAX_DISTANCE: usize = 10000;

#[derive(Debug, Copy, Clone)]
struct WordStats {
    // The last position (in words processed) that this word was seen at.
    last_seen_at: usize,

    // How many times this word has been seen so far.
    times_seen: usize,

    // Maximum distance before helps is needed again.
    max_distance: usize,
}

pub struct Learner {
    stats: HashMap<String, WordStats>,
    words_processed: usize,
    times_seen_threshold: usize,
}

impl Learner {
    pub fn new(times_seen_threshold: usize) -> Self {
        Self {
            stats: HashMap::new(),
            words_processed: 0,
            times_seen_threshold: times_seen_threshold,
        }
    }

    pub fn record(&mut self, word: &str) {
        self.stats
            .entry(word.to_string())
            .and_modify(|stats| {
                let distance = self.words_processed - stats.last_seen_at;

                stats.last_seen_at = self.words_processed;
                stats.times_seen += 1;
                if stats.times_seen <= self.times_seen_threshold {
                    return;
                }

                if distance < stats.max_distance {
                    stats.max_distance += distance.min((stats.max_distance as f64 * 0.5) as usize);
                }

                stats.max_distance = stats.max_distance.min(MAX_MAX_DISTANCE);
            })
            .or_insert(WordStats {
                last_seen_at: self.words_processed,
                times_seen: 1,
                max_distance: MIN_MAX_DISTANCE,
            });
        self.words_processed += 1;
    }

    pub fn needs_help(&self, word: &str) -> bool {
        if let Some(stats) = self.stats.get(word) {
            let distance = self.words_processed - stats.last_seen_at;
            stats.times_seen <= self.times_seen_threshold || distance > stats.max_distance
        } else {
            true
        }
    }
}

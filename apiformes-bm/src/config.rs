use crate::Arc;
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Sleep {
    NoDelay,
    ConstantTime(Duration),
    MinMax(Duration, Duration),
}

pub struct Config {
    pub endpoint: String,
    pub topic: Arc<str>,
    pub n_pubs: usize,
    pub n_subs: usize,
    pub sleep: Sleep,
    pub iterations: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            endpoint: "0.0.0.0:1883".to_owned(),
            topic: "/benchmark/stress_test".into(),
            n_pubs: 10,
            n_subs: 10,
            iterations: 1000,
            sleep: Sleep::ConstantTime(Duration::from_millis(1)),
        }
    }
}

use crate::{client::Client, config::Sleep};
use apiformes::packets::prelude::*;
use bytes::Bytes;
use rand::{distributions::Uniform, rngs::SmallRng, Rng, SeedableRng};
use std::io::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::ToSocketAddrs;
use tokio::sync::Notify;
use tokio::time::sleep;

pub struct PublisherStats {
    pub total_time: Duration,
    pub deltas: Vec<Duration>,
}

pub struct Publisher {
    time_reference: Instant,
    client: Client,
    topic: Arc<str>,
    iterations: usize,
    deltas: Vec<Duration>,
    sleep: Sleep,
    // the problem is that is the Publisher sends data too quickly
    // it might be done too quickly and as result it will shutdown
    // the connection, the problem is that the way we implement our
    // server, we place limits on how much data will it buffer,
    // as result, once connection is dropped some data may be lost
    // sd we must drop the connection only after all the Subscribers
    // receive the message.
    release_signal: Arc<Notify>,
}

impl Publisher {
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        topic: &str,
        iterations: usize,
        time_reference: Instant,
        sleep: Sleep,
        release_signal: Arc<Notify>,
    ) -> Result<Publisher> {
        Ok(Publisher {
            client: Client::new(addr).await?,
            topic: topic.into(),
            deltas: Vec::with_capacity(iterations),
            iterations,
            time_reference,
            sleep,
            release_signal,
        })
    }
    async fn run_nodelay(&mut self) -> Result<()> {
        for _ in 0..self.iterations {
            let start = Instant::now();
            let timestamp = start.duration_since(self.time_reference).as_nanos();
            let payload = timestamp.to_be_bytes();
            let bytes = Bytes::copy_from_slice(&payload[..]);
            let pub_packet = Publish::new(self.topic.clone(), bytes).unwrap().build();
            self.client.send(&pub_packet).await?;
            self.deltas.push(Instant::now().duration_since(start));
        }
        Ok(())
    }

    async fn run_constant_sleep(&mut self, sleep_time: Duration) -> Result<()> {
        for _ in 0..self.iterations {
            let start = Instant::now();
            let timestamp = start.duration_since(self.time_reference);
            let payload = timestamp.as_micros().to_be_bytes();
            let bytes = Bytes::copy_from_slice(&payload[..]);
            let pub_packet = Publish::new(self.topic.clone(), bytes).unwrap().build();
            self.client.send(&pub_packet).await?;
            self.deltas.push(Instant::now().duration_since(start));
            sleep(sleep_time).await;
        }
        Ok(())
    }
    async fn run_min_max_sleep(
        &mut self,
        min_sleep_time: Duration,
        max_sleep_time: Duration,
    ) -> Result<()> {
        let mut small_rng = SmallRng::from_entropy();
        let sample_space = Uniform::new(min_sleep_time, max_sleep_time);
        for _ in 0..self.iterations {
            let start = Instant::now();
            let timestamp = start.duration_since(self.time_reference).as_micros();
            let payload = timestamp.to_be_bytes();
            let bytes = Bytes::copy_from_slice(&payload[..]);
            let pub_packet = Publish::new(self.topic.clone(), bytes).unwrap().build();
            self.client.send(&pub_packet).await?;
            self.deltas.push(Instant::now().duration_since(start));
            sleep(small_rng.sample(sample_space)).await;
        }
        Ok(())
    }

    pub async fn run(mut self) -> Result<PublisherStats> {
        let start = Instant::now();
        let mut conn = Connect::new("".into()).unwrap();
        conn.set_clean_start();
        let conn = conn.build();
        self.client.send(&conn).await?;
        // we ignore receiving the ack message
        match self.sleep {
            Sleep::NoDelay => self.run_nodelay().await?,
            Sleep::ConstantTime(d) => self.run_constant_sleep(d).await?,
            Sleep::MinMax(min, max) => self.run_min_max_sleep(min, max).await?,
        }
        let total_time = Instant::now().duration_since(start);
        self.release_signal.notified().await;
        Ok(PublisherStats {
            total_time,
            deltas: self.deltas,
        })
    }
}

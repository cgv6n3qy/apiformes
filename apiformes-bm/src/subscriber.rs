use super::client::Client;
use apiformes_packet::prelude::*;
use std::io::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::ToSocketAddrs;

pub struct SubscriberStats {
    pub total_time: Duration,
    pub deltas: Vec<Duration>,
    pub trips_time: Vec<Duration>,
}

pub struct Subscriber {
    client: Client,
    time_reference: Instant,
    topic: Arc<str>,
    iterations: usize,
    deltas: Vec<Duration>,
    trips_time: Vec<Duration>,
}

impl Subscriber {
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        topic: Arc<str>,
        iterations: usize,
        time_reference: Instant,
    ) -> Result<Subscriber> {
        Ok(Subscriber {
            time_reference,
            client: Client::new(addr).await?,
            topic: topic,
            deltas: Vec::with_capacity(iterations),
            iterations,
            trips_time: Vec::with_capacity(iterations),
        })
    }

    async fn listen(&mut self) -> Result<()> {
        for _ in 0..self.iterations {
            let start = Instant::now();
            let packet = self.client.recv().await?;
            let p = match packet {
                Packet::Publish(p) => p,
                _ => continue,
            };
            let payload = p.payload();
            let t = u128::from_be_bytes(payload[..].try_into().unwrap());
            // Both sender and receiver have time reference t
            //  Reference                                              Now
            //  |---------------------------|---------------------------|
            //  |    t(stored in payload)   |      we need this part    |
            let now = Instant::now();
            let t_secs = (t / 1000000) as u64;
            let t_nanos = (t % 1000000 * 1000) as u32;
            let t = Duration::new(t_secs, t_nanos);
            let trip_time = now.duration_since(self.time_reference).saturating_sub(t);
            self.trips_time.push(trip_time);
            self.deltas.push(now.duration_since(start));
        }
        Ok(())
    }

    pub async fn connect(&mut self) -> Result<()> {
        let mut conn = Connect::new("".into()).unwrap();
        conn.set_clean_start();
        let conn = conn.build();
        self.client.send(&conn).await?;

        drop(self.client.recv().await?); // the ack message

        let mut packet = Subscribe::new(1);
        packet
            .add_topic(self.topic.clone(), RetainHandling::DoNotSend.into())
            .unwrap();
        let packet = packet.build();
        self.client.send(&packet).await?;
        drop(self.client.recv().await?); // the ack message
        Ok(())
    }
    pub async fn run(mut self) -> Result<SubscriberStats> {
        let start = Instant::now();
        self.listen().await?;
        Ok(SubscriberStats {
            total_time: Instant::now().duration_since(start),
            trips_time: self.trips_time,
            deltas: self.deltas,
        })
    }
}

use apiformes::packets::prelude::*;
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use std::io::Result;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct SubscriberStats {
    pub total_time: Duration,
    pub deltas: Vec<Duration>,
    pub trips_time: Vec<Duration>,
}

pub struct Subscriber {
    time_reference: Instant,
    stream: TcpStream,
    topic: String,
    iterations: usize,
    deltas: Vec<Duration>,
    trips_time: Vec<Duration>,
    bytes: BytesMut,
}

impl Subscriber {
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        topic: &str,
        iterations: usize,
        time_reference: Instant,
    ) -> Result<Subscriber> {
        Ok(Subscriber {
            time_reference,
            stream: TcpStream::connect(addr).await?,
            topic: topic.to_owned(),
            deltas: Vec::with_capacity(iterations),
            iterations,
            trips_time: Vec::with_capacity(iterations),
            bytes: BytesMut::with_capacity(128),
        })
    }
    async fn recv(&mut self) -> Result<Packet> {
        loop {
            let mut cursor = Cursor::new(&self.bytes[..]);
            match Packet::from_bytes(&mut cursor) {
                Ok(packet) => {
                    self.bytes.advance(packet.frame_len());
                    return Ok(packet);
                }
                Err(DataParseError::InsufficientBuffer {
                    needed: _,
                    available: _,
                }) => self.stream.read_buf(&mut self.bytes).await?,
                Err(e) => panic!("{:?}", e),
            };
        }
    }
    async fn listen(&mut self) -> Result<()> {
        for _ in 0..self.iterations {
            let start = Instant::now();
            let packet = self.recv().await?;
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
        let mut frame = BytesMut::with_capacity(128);
        let mut conn = Connect::new("".to_owned()).unwrap();
        conn.set_clean_start();
        conn.build().to_bytes(&mut frame).unwrap();
        self.stream.write_all_buf(&mut frame).await?;

        drop(self.recv().await?); // the ack message

        let mut packet = Subscribe::new(1);
        packet
            .add_topic(&self.topic, RetainHandling::DoNotSend.into())
            .unwrap();
        packet.build().to_bytes(&mut frame).unwrap();
        self.stream.write_all_buf(&mut frame).await?;
        drop(self.recv().await?); // the ack message
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

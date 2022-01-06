#[macro_use]
extern crate clap;

mod client;
mod config;
mod publisher;
mod subscriber;

use clap::App;
use config::*;
use futures::future::{join_all, JoinAll};
use publisher::*;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use subscriber::*;
use tokio::sync::Notify;
use tokio::time::sleep;
use tokio::{select, task::JoinHandle};

async fn starts_subs(cfg: &Config, time_ref: Instant) -> JoinAll<JoinHandle<SubscriberStats>> {
    let mut subs_handles = Vec::with_capacity(cfg.n_subs);
    for _ in 0..cfg.n_subs {
        // each subscriber would receive iteration * n_pubs messages
        let mut sub = Subscriber::new(
            &cfg.endpoint,
            cfg.topic.clone(),
            cfg.iterations * cfg.n_pubs,
            time_ref,
        )
        .await
        .unwrap();
        sub.connect().await.unwrap();
        subs_handles.push(tokio::spawn(async move { sub.run().await.unwrap() }));
    }
    join_all(subs_handles)
}

async fn start_pubs(
    cfg: &Config,
    time_ref: Instant,
) -> (JoinAll<JoinHandle<PublisherStats>>, Arc<Notify>) {
    let release_signal = Arc::new(Notify::new());
    let mut pubs_handles = Vec::with_capacity(cfg.n_pubs);
    for _ in 0..cfg.n_pubs {
        let _pub = Publisher::new(
            &cfg.endpoint,
            &cfg.topic,
            cfg.iterations,
            time_ref,
            cfg.sleep,
            release_signal.clone(),
        )
        .await
        .unwrap();
        pubs_handles.push(tokio::spawn(async move { _pub.run().await.unwrap() }));
    }
    (join_all(pubs_handles), release_signal)
}

fn str_to_duration(input: &str) -> Duration {
    if input.ends_with("us") {
        //microseconds
        Duration::from_micros(
            input
                .get(0..input.len() - "us".len())
                .unwrap()
                .parse()
                .unwrap(),
        )
    } else if input.ends_with("µs") {
        // also microseconds
        Duration::from_micros(
            input
                .get(0..input.len() - "µs".len())
                .unwrap()
                .parse()
                .unwrap(),
        )
    } else if input.ends_with("ms") {
        //milliseconds
        Duration::from_millis(
            input
                .get(0..input.len() - "ms".len())
                .unwrap()
                .parse()
                .unwrap(),
        )
    } else if input.ends_with('s') {
        // seconds
        Duration::from_secs(
            input
                .get(0..input.len() - "s".len())
                .unwrap()
                .parse()
                .unwrap(),
        )
    } else {
        panic!("failed to parse duration");
    }
}

#[tokio::main]
async fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml)
        .version(crate_version!())
        .version_short("v")
        .get_matches();

    let mut cfg: Config = Default::default();
    if let Some(endpoint) = matches.value_of("Endpoint") {
        cfg.endpoint = endpoint.to_owned();
    }
    if let Some(n_subs) = matches.value_of("Subscribers") {
        cfg.n_subs = n_subs.parse().unwrap();
    }
    if let Some(n_pubs) = matches.value_of("Publishers") {
        cfg.n_pubs = n_pubs.parse().unwrap();
    }
    if let Some(iterations) = matches.value_of("Messages") {
        cfg.iterations = iterations.parse().unwrap();
    }
    if let Some(topic) = matches.value_of("Topic") {
        cfg.topic = topic.into();
    }
    if matches.is_present("NoDelay") {
        cfg.sleep = Sleep::NoDelay;
    } else if let Some(delay) = matches.value_of("ConstDelay") {
        cfg.sleep = Sleep::ConstantTime(str_to_duration(delay));
    } else if let Some(delay) = matches.value_of("MinMaxDelay") {
        let delays: Vec<_> = delay.split(':').collect();
        if delays.len() != 2 {
            panic!("Failed to parse `min:max` style duration");
        }
        cfg.sleep = Sleep::MinMax(str_to_duration(delays[0]), str_to_duration(delays[1]));
    }

    println!("Benchmarking configuration:");
    println!("Number of concurrent Publishers: {}", cfg.n_pubs);
    println!("Number of concurrent Subscribers: {}", cfg.n_subs);
    println!("Benchmarking topic: {}", cfg.topic);
    println!("Number of publish messages: {}", cfg.iterations);

    let time_ref = Instant::now();

    // first start the subscribers because if we start publishers first some messages may not be
    // delivered at all
    let subs_handles = starts_subs(&cfg, time_ref).await;

    // next we start the publishers this way we are sure that all published messages will be captured
    let (mut pubs_handles, release_signal) = start_pubs(&cfg, time_ref).await;

    // we then wait for all subscribers to finish
    let subs = subs_handles.await;

    // notify all waiting threads, and allow them to join...
    let pubs;
    loop {
        release_signal.notify_waiters();
        release_signal.notify_one();
        select! {
            _ = sleep(Duration::from_millis(1)) => (),
            p = &mut pubs_handles => {
                pubs = p;
                break;
            }
        };
    }
    let publishing_rate: f64 = pubs
        .iter()
        .map(|s| {
            cfg.iterations as f64
                / s.as_ref()
                    .unwrap()
                    .deltas
                    .iter()
                    .sum::<Duration>()
                    .as_micros() as f64
        })
        .sum::<f64>()
        / cfg.n_pubs as f64
        * 1000000.0; //because we are doing measurements in micrseconds

    let arrival_rate: f64 = subs
        .iter()
        .map(|s| {
            (cfg.n_pubs * cfg.iterations) as f64
                / s.as_ref()
                    .unwrap()
                    .deltas
                    .iter()
                    .sum::<Duration>()
                    .as_micros() as f64
        })
        .sum::<f64>()
        / cfg.n_pubs as f64
        * 1000000.0; //because we are doing measurements in micrseconds

    let mut aggregate: Vec<_> = subs
        .into_iter()
        .map(|s| s.unwrap().trips_time)
        .flatten()
        .collect();
    aggregate.sort();
    println!(
        "Mean departure rate = {:.2} packets/second/publisher",
        publishing_rate
    );
    println!(
        "Mean arrival rate = {:.2} packets/second/subscriber",
        arrival_rate
    );
    println!("Minimum trip time: {:?}", aggregate[0]);
    println!("Maximum trip time: {:?}", aggregate[aggregate.len() - 1]);
    println!(
        "75th percentile trip time: {:?}",
        aggregate[aggregate.len() * 75 / 100]
    );
    println!(
        "90th percentile trip time: {:?}",
        aggregate[aggregate.len() * 90 / 100]
    );
    println!(
        "99th percentile trip time: {:?}",
        aggregate[aggregate.len() * 99 / 100]
    );
}

//! Focused cost probe, not a performance gate or a cross-router comparison.

use http::Request;
use std::{hint::black_box, time::Instant};
use warded::{Head, Selector, negotiation::Offers};

fn main() {
    const ITERATIONS: u32 = 50;
    println!(
        "shared-offer last-hit probe; 257 header members; {ITERATIONS} iterations; release profile"
    );
    for count in [1, 16, 64, 128] {
        let values: Vec<_> = (0..count).map(|i| format!("application/x-{i}")).collect();
        let offers = Offers::media_types(values.iter().cloned()).unwrap();
        let mut routes = Selector::new();
        for (index, offer) in values.iter().enumerate() {
            routes = routes.route(offers.selected(offer).unwrap(), index);
        }
        let mut members: Vec<_> = (0..256)
            .map(|i| format!("application/a{i};q=0.5"))
            .collect();
        members.push(values[count - 1].clone());
        let header = members.join(",");
        let request = Request::builder()
            .header("accept", &header)
            .body(())
            .unwrap();
        let select = || {
            let head = Head::from_request(black_box(&request));
            let selected = routes.select(&head).unwrap().unwrap();
            assert_eq!(*black_box(selected.target), count - 1);
        };
        for _ in 0..5 {
            select();
        }
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            select();
        }
        println!(
            "routes={count}; bytes={}; mean_us={:.3}",
            header.len(),
            start.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(ITERATIONS)
        );
    }
}

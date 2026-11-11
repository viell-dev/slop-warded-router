//! Focused pre-freeze cost probe. Correctness assertions are part of each sample.
//! No throughput promise, cross-router comparison, or CI timing threshold.

use http::Request;
use std::{hint::black_box, time::Instant};
use warded::{
    Head, Selector,
    matchers::path,
    negotiation::{Offers, accepts_media_type},
};

fn measure(
    routes: &Selector<usize>,
    request: &Request<()>,
    expected: Option<usize>,
    capture: bool,
    iterations: u32,
) -> f64 {
    let select = || {
        // Include creation of a fresh request-head cache for every selection.
        let head = Head::from_request(black_box(request));
        let selected = routes.select(&head).unwrap();
        assert_eq!(selected.as_ref().map(|selected| *selected.target), expected);
        if capture && let Some(selected) = &selected {
            assert_eq!(selected.selection.captures.get("id"), Some("raw%2Fvalue"));
        }
        black_box(selected);
    };
    for _ in 0..3 {
        select();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        select();
    }
    start.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(iterations)
}

fn main() {
    println!("strategy,routes,field_bytes,case,iterations,mean_us");
    for count in [16, 128, 1024] {
        let mut routes = Selector::new();
        for index in 0..count {
            routes = routes.route(path(format!("/route/{index}/{{id}}")).unwrap(), index);
        }
        for (name, expected) in [
            ("early", Some(0)),
            ("late", Some(count - 1)),
            ("miss", None),
        ] {
            let request = Request::builder()
                .uri(format!("/route/{}/raw%2Fvalue", expected.unwrap_or(count)))
                .body(())
                .unwrap();
            let mean = measure(&routes, &request, expected, true, 1000);
            println!("path,{count},0,{name},1000,{mean:.3}");
        }
    }

    for count in [16, 128] {
        let values: Vec<_> = (0..count)
            .map(|index| format!("application/x-{index}"))
            .collect();
        let offers = Offers::media_types(values.iter().cloned()).unwrap();
        let mut shared = Selector::new();
        let mut independent = Selector::new();
        for (index, offer) in values.iter().enumerate() {
            shared = shared.route(offers.selected(offer).unwrap(), index);
            independent = independent.route(accepts_media_type(offer).unwrap(), index);
        }
        for members in [4, 1023] {
            let unsupported: Vec<_> = (0..members)
                .map(|index| format!("application/{index:04}{};q=0.5", "x".repeat(37)))
                .collect();
            for (name, expected) in [
                ("early", Some(0)),
                ("late", Some(count - 1)),
                ("miss", None),
            ] {
                let mut field = unsupported.join(",");
                if let Some(index) = expected {
                    field.push(',');
                    field.push_str(&values[index]);
                }
                assert!(field.len() < 65_536);
                let request = Request::builder()
                    .header("accept", &field)
                    .body(())
                    .unwrap();
                for (strategy, routes) in
                    [("shared-offers", &shared), ("independent", &independent)]
                {
                    let mean = measure(routes, &request, expected, false, 20);
                    println!("{strategy},{count},{},{name},20,{mean:.3}", field.len());
                }
            }
        }
    }
}

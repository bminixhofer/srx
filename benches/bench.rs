use std::{fs, str::FromStr};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use srx::{Rules, SRX};

fn split<'a>(string: &'a str, rules: &Rules) -> Vec<&'a str> {
    rules.split(string)
}

fn criterion_benchmark(c: &mut Criterion) {
    let rules =
        SRX::from_str(&fs::read_to_string("data/example.srx").expect("example file exists"))
            .expect("example file is valid")
            .language_rules("en");

    c.bench_function("split string", |b| {
        b.iter(|| {
            split(
                black_box(
                    "The U.K. Prime Minister, Mr. Blair, was seen out with his family today.",
                ),
                &rules,
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

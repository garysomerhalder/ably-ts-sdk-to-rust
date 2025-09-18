# PROD-010: Implement Performance Benchmarking Suite

## 🎯 Objective
Create comprehensive benchmarking suite to validate performance claims and track regressions.

## 📋 Task Details

**Priority:** 🟢 LOW
**Effort:** 2 days
**Assignee:** Performance Engineer
**Dependencies:** All other tasks

## 🔍 Problem

No performance benchmarks exist to validate 40% performance improvement claim.

## ✅ Acceptance Criteria

1. [ ] Message throughput benchmarks
2. [ ] Connection latency measurements
3. [ ] Memory usage tracking
4. [ ] Comparison with JS SDK
5. [ ] CI integration for regression detection

## 🛠️ Implementation

### `benches/sdk_benchmarks.rs`
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_message_publish(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client = RestClient::new(API_KEY);

    let mut group = c.benchmark_group("message_publish");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("single", size),
            size,
            |b, &size| {
                let message = create_message(*size);
                b.to_async(&runtime).iter(|| async {
                    let channel = client.channel("bench");
                    channel.publish(black_box(message.clone())).await
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("batch", size),
            size,
            |b, &size| {
                let messages = vec![create_message(*size); 10];
                b.to_async(&runtime).iter(|| async {
                    let channel = client.channel("bench");
                    channel.publish_batch(black_box(messages.clone())).await
                });
            },
        );
    }
    group.finish();
}

fn benchmark_websocket_latency(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("websocket_roundtrip", |b| {
        b.to_async(&runtime).iter(|| async {
            let client = RealtimeClient::new(API_KEY);
            client.connect().await.unwrap();

            let start = Instant::now();
            let channel = client.channel("latency-test").await;
            channel.attach().await.unwrap();

            let message = Message {
                name: Some("ping".to_string()),
                data: Some(json!({"timestamp": start.elapsed().as_micros()})),
                ..Default::default()
            };

            channel.publish(message).await.unwrap();
            black_box(start.elapsed())
        });
    });
}

fn benchmark_memory_usage(c: &mut Criterion) {
    c.bench_function("1000_channels_memory", |b| {
        b.iter(|| {
            let client = RestClient::new(API_KEY);
            let channels: Vec<_> = (0..1000)
                .map(|i| client.channel(&format!("channel-{}", i)))
                .collect();

            // Measure memory
            let mem_usage = get_memory_usage();
            black_box((channels, mem_usage))
        });
    });
}

fn benchmark_connection_recovery(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("connection_recovery_time", |b| {
        b.to_async(&runtime).iter(|| async {
            let client = RealtimeClient::new(API_KEY);
            client.connect().await.unwrap();

            // Simulate disconnect
            client.disconnect().await;

            let start = Instant::now();
            client.connect().await.unwrap();
            black_box(start.elapsed())
        });
    });
}

criterion_group!(
    benches,
    benchmark_message_publish,
    benchmark_websocket_latency,
    benchmark_memory_usage,
    benchmark_connection_recovery
);
criterion_main!(benches);
```

### Comparison Script
```rust
// benches/compare_with_js.rs
async fn compare_sdk_performance() {
    println!("Performance Comparison: Rust SDK vs JS SDK");
    println!("=" * 50);

    // Message Publishing
    let rust_publish_time = benchmark_rust_publish().await;
    let js_publish_time = benchmark_js_publish().await;
    let improvement = ((js_publish_time - rust_publish_time) / js_publish_time) * 100.0;

    println!("Message Publishing:");
    println!("  Rust SDK: {:.2}ms", rust_publish_time);
    println!("  JS SDK: {:.2}ms", js_publish_time);
    println!("  Improvement: {:.1}%", improvement);

    // Memory Usage
    let rust_memory = measure_rust_memory().await;
    let js_memory = measure_js_memory().await;
    let memory_saving = ((js_memory - rust_memory) as f64 / js_memory as f64) * 100.0;

    println!("\nMemory Usage (1000 channels):");
    println!("  Rust SDK: {}MB", rust_memory);
    println!("  JS SDK: {}MB", js_memory);
    println!("  Saving: {:.1}%", memory_saving);

    // Connection Latency
    let rust_connect = benchmark_rust_connection().await;
    let js_connect = benchmark_js_connection().await;

    println!("\nConnection Latency:");
    println!("  Rust SDK: {:.2}ms", rust_connect);
    println!("  JS SDK: {:.2}ms", js_connect);
    println!("  Improvement: {:.1}%", ((js_connect - rust_connect) / js_connect) * 100.0);
}
```

### CI Integration
```yaml
# .github/workflows/benchmark.yml
name: Benchmarks

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run benchmarks
        run: cargo bench --all-features -- --output-format bencher | tee output.txt

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          alert-threshold: '110%'
          comment-on-alert: true
          fail-on-alert: true
```

## 📊 Success Metrics

- ✅ Proves 40% performance improvement
- ✅ Memory usage 50% lower than JS
- ✅ Automated regression detection
- ✅ Benchmark results in CI
- ✅ Performance dashboard available
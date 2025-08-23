use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kilar::port::{
    adaptive::{AdaptivePortManager, PerformanceProfile},
    incremental::IncrementalPortManager,
    PortManager,
};
use std::time::Duration;
use tokio::runtime::Runtime;

fn benchmark_legacy_list(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("legacy_list_tcp", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let result = port_manager.list_processes("tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_adaptive_fast(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("adaptive_fast_tcp", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = AdaptivePortManager::new(PerformanceProfile::Fast);
            let result = port_manager.list_processes("tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_adaptive_balanced(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("adaptive_balanced_tcp", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = AdaptivePortManager::new(PerformanceProfile::Balanced);
            let result = port_manager.list_processes("tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_incremental(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("incremental_tcp", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = IncrementalPortManager::new(PerformanceProfile::Fast);
            let result = port_manager.get_processes("tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_check_port_legacy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("check_port_legacy", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let result = port_manager.check_port(black_box(8080), "tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_check_port_adaptive(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("check_port_adaptive", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = AdaptivePortManager::new(PerformanceProfile::Fast);
            let result = port_manager.check_port(black_box(8080), "tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_check_port_incremental(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("check_port_incremental", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = IncrementalPortManager::new(PerformanceProfile::Fast);
            let result = port_manager.get_port(black_box(8080), "tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_multiple_protocols(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("legacy_all_protocols", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let tcp_result = port_manager.list_processes("tcp").await;
            let udp_result = port_manager.list_processes("udp").await;
            black_box((tcp_result, udp_result))
        });
    });

    c.bench_function("adaptive_all_protocols", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = AdaptivePortManager::new(PerformanceProfile::Fast);
            let tcp_result = port_manager.list_processes("tcp").await;
            let udp_result = port_manager.list_processes("udp").await;
            black_box((tcp_result, udp_result))
        });
    });
}

fn benchmark_cache_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("incremental_cache_cold", |b| {
        b.to_async(&rt).iter(|| async {
            let mut port_manager = IncrementalPortManager::new(PerformanceProfile::Balanced);
            port_manager.force_refresh().await;
            let result = port_manager.get_processes("tcp").await;
            black_box(result)
        });
    });

    c.bench_function("incremental_cache_warm", |b| {
        b.to_async(&rt).iter_batched_ref(
            || {
                let mut manager = IncrementalPortManager::new(PerformanceProfile::Balanced);
                rt.block_on(async {
                    let _ = manager.get_processes("tcp").await;
                });
                manager
            },
            |port_manager| async {
                let result = port_manager.get_processes("tcp").await;
                black_box(result)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    benchmark_legacy_list,
    benchmark_adaptive_fast,
    benchmark_adaptive_balanced,
    benchmark_incremental,
    benchmark_check_port_legacy,
    benchmark_check_port_adaptive,
    benchmark_check_port_incremental,
    benchmark_multiple_protocols,
    benchmark_cache_efficiency
);
criterion_main!(benches);

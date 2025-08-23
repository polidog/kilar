use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kilar::port::PortManager;
use tokio::runtime::Runtime;

fn benchmark_port_manager_list_tcp(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("port_manager_list_tcp", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let result = port_manager.list_processes("tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_port_manager_list_udp(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("port_manager_list_udp", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let result = port_manager.list_processes("udp").await;
            black_box(result)
        });
    });
}

fn benchmark_check_port(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("check_port", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let result = port_manager.check_port(black_box(8080), "tcp").await;
            black_box(result)
        });
    });
}

fn benchmark_multiple_protocols(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("all_protocols", |b| {
        b.to_async(&rt).iter(|| async {
            let port_manager = PortManager::new();
            let tcp_result = port_manager.list_processes("tcp").await;
            let udp_result = port_manager.list_processes("udp").await;
            black_box((tcp_result, udp_result))
        });
    });
}

criterion_group!(
    benches,
    benchmark_port_manager_list_tcp,
    benchmark_port_manager_list_udp,
    benchmark_check_port,
    benchmark_multiple_protocols
);
criterion_main!(benches);

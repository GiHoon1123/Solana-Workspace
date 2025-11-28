use std::time::Instant;

use anyhow::Result;
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_decimal::Decimal;
use tokio::runtime::Runtime;

use api_server::domains::cex::engine::runtime::HighPerformanceEngine;
use api_server::domains::cex::engine::types::OrderEntry;
use api_server::domains::cex::engine::Engine;

#[path = "../tests/common/mod.rs"]
mod bench_common;

use bench_common::{setup_test_with_orderbook, teardown_test, NUM_TEST_USERS};

const ORDER_BATCHES: [usize; 4] = [1_000, 5_000, 10_000, 50_000];

fn bench_limit_order_tps(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime");
    let mut group = c.benchmark_group("limit_order_tps");

    for &order_count in ORDER_BATCHES.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(order_count),
            &order_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let (mut engine, db) = setup_test_with_orderbook(false).await;

                        let start = Instant::now();
                        submit_limit_orders(&mut engine, count)
                            .await
                            .expect("failed to submit orders");
                        let elapsed = start.elapsed();

                        teardown_test(&mut engine, &db).await;
                        black_box(elapsed)
                    })
                });
            },
        );
    }

    group.finish();
}

async fn submit_limit_orders(engine: &mut HighPerformanceEngine, total: usize) -> Result<()> {
    for idx in 0..total {
        let order_id = 1_000_000 + idx as u64;
        let user_id = (idx as u64 % NUM_TEST_USERS) + 1;
        let amount = Decimal::new(10, 1); // 1.0 SOL
        let price = Decimal::new(10_000 + (idx as i64 % 50) * 10, 2);
        let is_buy = idx % 2 == 0;

        let order = build_limit_order(order_id, user_id, price, amount, is_buy);
        engine.submit_order(order).await?;
    }

    Ok(())
}

fn build_limit_order(
    order_id: u64,
    user_id: u64,
    price: Decimal,
    amount: Decimal,
    is_buy: bool,
) -> OrderEntry {
    OrderEntry {
        id: order_id,
        user_id,
        order_type: if is_buy { "buy" } else { "sell" }.to_string(),
        order_side: "limit".to_string(),
        base_mint: "SOL".to_string(),
        quote_mint: "USDT".to_string(),
        price: Some(price),
        amount,
        quote_amount: None,
        filled_amount: Decimal::ZERO,
        remaining_amount: amount,
        remaining_quote_amount: None,
        created_at: Utc::now(),
    }
}

criterion_group!(benches, bench_limit_order_tps);
criterion_main!(benches);



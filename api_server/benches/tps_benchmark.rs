// =====================================================
// TPS 벤치마크 테스트
// =====================================================
// 목적: 엔진의 초당 주문 처리량(Transactions Per Second) 측정
// 
// 테스트 시나리오:
// 1. 엔진 초기화 및 시작
// 2. 초기 잔고 설정 (100명의 사용자, 각각 SOL과 USDT 보유)
// 3. 대량의 주문 생성 및 제출
// 4. 처리 시간 측정 및 TPS 계산
// 
// 목표 TPS: 100,000 orders/sec 이상
// =====================================================

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};
use rust_decimal::Decimal;
use chrono::Utc;
use tokio::runtime::Runtime;

use api_server::shared::database::Database;
use api_server::domains::cex::engine::runtime::HighPerformanceEngine;
use api_server::domains::cex::engine::types::OrderEntry;

// 테스트용 상수
const BASE_DATABASE_URL: &str = "postgresql://root:1234@localhost/solana_api_test";
const NUM_USERS: u64 = 100;

// 초기 잔고 (함수로 생성)
fn initial_sol_balance() -> Decimal {
    Decimal::new(10000, 0)  // 10,000 SOL
}

fn initial_usdt_balance() -> Decimal {
    Decimal::new(10000000, 0)  // 10,000,000 USDT
}

/// 엔진 초기화 및 시작
/// 
/// # Returns
/// * `HighPerformanceEngine` - 시작된 엔진
/// * `Runtime` - Tokio 런타임
async fn setup_engine() -> anyhow::Result<(HighPerformanceEngine, Runtime)> {
    // 데이터베이스 연결
    let db = Database::new(BASE_DATABASE_URL)
        .await
        .expect("Failed to connect to database");
    
    // 마이그레이션 실행
    db.initialize()
        .await
        .expect("Failed to initialize database");
    
    // 초기 데이터 정리 (테스트 격리)
    cleanup_test_data(&db).await;
    
    // 초기 잔고 설정
    setup_initial_balances(&db).await?;
    
    // 엔진 생성
    let mut engine = HighPerformanceEngine::new(db);
    
    // 엔진 시작
    engine.start().await?;
    
    // Tokio 런타임 생성 (벤치마크에서 사용)
    let rt = Runtime::new().unwrap();
    
    Ok((engine, rt))
}

/// 테스트 데이터 정리
/// 
/// 이전 테스트에서 남은 데이터를 삭제하여 테스트 격리 보장
async fn cleanup_test_data(db: &Database) {
    use sqlx::query;
    
    let pool = db.pool();
    
    // 트랜잭션으로 모든 테스트 데이터 삭제
    let mut tx = pool.begin().await.unwrap();
    
    use sqlx::query;
    query("DELETE FROM trades").execute(&mut *tx).await.unwrap();
    query("DELETE FROM orders").execute(&mut *tx).await.unwrap();
    query("DELETE FROM user_balances").execute(&mut *tx).await.unwrap();
    
    tx.commit().await.unwrap();
}

/// 초기 잔고 설정
/// 
/// 테스트용 사용자들의 잔고를 설정합니다.
/// 각 사용자는 SOL과 USDT를 보유합니다.
async fn setup_initial_balances(db: &Database) -> anyhow::Result<()> {
    use sqlx::query;
    
    let pool = db.pool();
    
    // 트랜잭션으로 모든 잔고 삽입
    let mut tx = pool.begin().await?;
    
    use sqlx::query;
    for user_id in 1..=NUM_USERS {
        // SOL 잔고
        query(
            r#"
            INSERT INTO user_balances (user_id, mint, available, locked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (user_id, mint) DO UPDATE
            SET available = $3, locked = $4, updated_at = NOW()
            "#,
        )
        .bind(user_id as i64)
        .bind("SOL")
        .bind(initial_sol_balance())
        .bind(Decimal::ZERO)
        .execute(&mut *tx)
        .await?;
        
        // USDT 잔고
        query(
            r#"
            INSERT INTO user_balances (user_id, mint, available, locked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (user_id, mint) DO UPDATE
            SET available = $3, locked = $4, updated_at = NOW()
            "#,
        )
        .bind(user_id as i64)
        .bind("USDT")
        .bind(initial_usdt_balance())
        .bind(Decimal::ZERO)
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    
    Ok(())
}

/// 주문 생성 (지정가 매수)
/// 
/// 테스트용 지정가 매수 주문을 생성합니다.
fn create_limit_buy_order(order_id: u64, user_id: u64, price: Decimal, amount: Decimal) -> OrderEntry {
    OrderEntry {
        id: order_id,
        user_id,
        order_type: "buy".to_string(),
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

/// 주문 생성 (지정가 매도)
/// 
/// 테스트용 지정가 매도 주문을 생성합니다.
fn create_limit_sell_order(order_id: u64, user_id: u64, price: Decimal, amount: Decimal) -> OrderEntry {
    OrderEntry {
        id: order_id,
        user_id,
        order_type: "sell".to_string(),
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

/// 주문 생성 (시장가 매수 - 금액 기반)
/// 
/// 테스트용 시장가 매수 주문을 생성합니다.
/// quote_amount만 사용합니다 (금액 기반).
fn create_market_buy_order(order_id: u64, user_id: u64, quote_amount: Decimal) -> OrderEntry {
    OrderEntry {
        id: order_id,
        user_id,
        order_type: "buy".to_string(),
        order_side: "market".to_string(),
        base_mint: "SOL".to_string(),
        quote_mint: "USDT".to_string(),
        price: None,
        amount: Decimal::ZERO,  // 매칭 시 계산됨
        quote_amount: Some(quote_amount),
        filled_amount: Decimal::ZERO,
        remaining_amount: Decimal::ZERO,
        remaining_quote_amount: Some(quote_amount),
        created_at: Utc::now(),
    }
}

/// TPS 벤치마크: 지정가 주문 처리
/// 
/// 지정가 매수/매도 주문의 처리 속도를 측정합니다.
fn bench_limit_orders(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // 엔진 초기화 (한 번만)
    let (mut engine, _) = rt.block_on(setup_engine()).unwrap();
    
    let mut group = c.benchmark_group("limit_orders");
    
    // 다양한 주문 수로 테스트
    for order_count in [100, 1000, 10000, 50000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(order_count),
            order_count,
            |b, &order_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        
                        // 주문 생성 및 제출
                        for i in 0..order_count {
                            let order_id = 1000000 + i as u64;
                            let user_id = (i % NUM_USERS) + 1;
                            
                            // 매수/매도 번갈아가며 생성
                            if i % 2 == 0 {
                                // 매수 주문: 가격 100 + i%10, 수량 1.0
                                let price = Decimal::new(10000 + (i % 10) as i64 * 100, 2);
                                let amount = Decimal::new(10, 1);  // 1.0
                                let order = create_limit_buy_order(order_id, user_id, price, amount);
                                black_box(engine.submit_order(order).await.unwrap());
                            } else {
                                // 매도 주문: 가격 100 + i%10, 수량 1.0
                                let price = Decimal::new(10000 + (i % 10) as i64 * 100, 2);
                                let amount = Decimal::new(10, 1);  // 1.0
                                let order = create_limit_sell_order(order_id, user_id, price, amount);
                                black_box(engine.submit_order(order).await.unwrap());
                            }
                        }
                        
                        let elapsed = start.elapsed();
                        let tps = order_count as f64 / elapsed.as_secs_f64();
                        
                        // 결과 출력 (선택사항)
                        // println!("Processed {} orders in {:?}, TPS: {:.2}", order_count, elapsed, tps);
                        
                        elapsed
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// TPS 벤치마크: 시장가 매수 주문 처리
/// 
/// 시장가 매수 주문(금액 기반)의 처리 속도를 측정합니다.
fn bench_market_buy_orders(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // 엔진 초기화 (한 번만)
    let (mut engine, _) = rt.block_on(setup_engine()).unwrap();
    
    // 먼저 매도 주문을 오더북에 추가 (시장가 매수가 매칭될 수 있도록)
    rt.block_on(async {
        for i in 0..1000 {
            let order_id = 2000000 + i as u64;
            let user_id = (i % NUM_USERS) + 1;
            let price = Decimal::new(10000, 2);  // 100 USDT
            let amount = Decimal::new(10, 1);  // 1.0 SOL
            let order = create_limit_sell_order(order_id, user_id, price, amount);
            engine.submit_order(order).await.unwrap();
        }
    });
    
    let mut group = c.benchmark_group("market_buy_orders");
    
    // 다양한 주문 수로 테스트
    for order_count in [100, 1000, 10000, 50000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(order_count),
            order_count,
            |b, &order_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        
                        // 시장가 매수 주문 생성 및 제출
                        for i in 0..order_count {
                            let order_id = 3000000 + i as u64;
                            let user_id = (i % NUM_USERS) + 1;
                            
                            // 금액 기반 시장가 매수: 100 USDT어치
                            let quote_amount = Decimal::new(10000, 2);  // 100 USDT
                            let order = create_market_buy_order(order_id, user_id, quote_amount);
                            black_box(engine.submit_order(order).await.unwrap());
                        }
                        
                        let elapsed = start.elapsed();
                        let tps = order_count as f64 / elapsed.as_secs_f64();
                        
                        elapsed
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// TPS 벤치마크: 혼합 주문 처리
/// 
/// 지정가와 시장가 주문을 혼합하여 처리 속도를 측정합니다.
fn bench_mixed_orders(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // 엔진 초기화 (한 번만)
    let (mut engine, _) = rt.block_on(setup_engine()).unwrap();
    
    let mut group = c.benchmark_group("mixed_orders");
    
    // 다양한 주문 수로 테스트
    for order_count in [1000, 10000, 50000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(order_count),
            order_count,
            |b, &order_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        
                        // 혼합 주문 생성 및 제출
                        for i in 0..order_count {
                            let order_id = 4000000 + i as u64;
                            let user_id = (i % NUM_USERS) + 1;
                            
                            match i % 3 {
                                0 => {
                                    // 지정가 매수
                                    let price = Decimal::new(10000 + (i % 10) as i64 * 100, 2);
                                    let amount = Decimal::new(10, 1);
                                    let order = create_limit_buy_order(order_id, user_id, price, amount);
                                    black_box(engine.submit_order(order).await.unwrap());
                                }
                                1 => {
                                    // 지정가 매도
                                    let price = Decimal::new(10000 + (i % 10) as i64 * 100, 2);
                                    let amount = Decimal::new(10, 1);
                                    let order = create_limit_sell_order(order_id, user_id, price, amount);
                                    black_box(engine.submit_order(order).await.unwrap());
                                }
                                _ => {
                                    // 시장가 매수 (금액 기반)
                                    let quote_amount = Decimal::new(10000, 2);
                                    let order = create_market_buy_order(order_id, user_id, quote_amount);
                                    black_box(engine.submit_order(order).await.unwrap());
                                }
                            }
                        }
                        
                        let elapsed = start.elapsed();
                        let tps = order_count as f64 / elapsed.as_secs_f64();
                        
                        elapsed
                    })
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_limit_orders, bench_market_buy_orders, bench_mixed_orders);
criterion_main!(benches);


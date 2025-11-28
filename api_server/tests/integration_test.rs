// =====================================================
// 엔진 통합 테스트
// =====================================================
// 목적: 엔진의 전체 로직이 올바르게 동작하는지 검증
// 
// 테스트 순서:
// 1. 데이터베이스 연결
// 2. 마이그레이션 실행
// 3. 초기 잔고 생성
// 4. 엔진 생성 및 시작
// =====================================================

use rust_decimal::Decimal;
use api_server::shared::database::Database;
use api_server::domains::cex::engine::runtime::HighPerformanceEngine;
use api_server::domains::cex::engine::types::OrderEntry;
use api_server::domains::cex::engine::Engine;

// 테스트용 상수
const TEST_DATABASE_URL: &str = "postgresql://root:1234@localhost/solana_api_test";
const TEST_USER_ID: u64 = 1;  // 실제 테스트에 사용할 유저 ID
const NUM_TEST_USERS: u64 = 100;  // 오더북 채우기용 유저 수

// 초기 잔고 (함수로 생성)
fn initial_sol_balance() -> Decimal {
    Decimal::new(10000, 0)  // 10,000 SOL
}

fn initial_usdt_balance() -> Decimal {
    Decimal::new(10000000, 0)  // 10,000,000 USDT
}

/// 테스트 전 초기화
/// 
/// 데이터베이스 연결, 마이그레이션, 초기 잔고 설정을 순차적으로 수행합니다.
async fn setup_test() -> (HighPerformanceEngine, Database) {
    // 1. 데이터베이스 연결
    let db = Database::new(TEST_DATABASE_URL)
        .await
        .expect("Failed to connect to database");
    
    // 2. 마이그레이션 실행
    db.initialize()
        .await
        .expect("Failed to initialize database");
    
    // 3. 테스트 데이터 정리
    cleanup_test_data(&db).await;
    
    // 4. 초기 잔고 설정
    setup_test_balances(&db).await;
    
    // 5. 엔진 생성 및 시작
    let mut engine = HighPerformanceEngine::new(db.clone());
    engine.start().await.expect("Failed to start engine");
    
    // 6. 오더북 쌓기 (거래가 일어나지 않도록 가격 겹치지 않게 설정)
    setup_orderbook(&mut engine).await;
    
    (engine, db)
}

/// 테스트 후 정리
/// 
/// 엔진을 중지하고 테스트 데이터를 정리합니다.
async fn teardown_test(engine: &mut HighPerformanceEngine, db: &Database) {
    // 엔진 중지
    engine.stop().await.expect("Failed to stop engine");
    
    // 테스트 데이터 정리
    cleanup_test_data(db).await;
}

/// 테스트 데이터 정리
/// 
/// 이전 테스트에서 남은 데이터를 삭제합니다.
async fn cleanup_test_data(db: &Database) {
    use sqlx::query;
    
    let pool = db.pool();
    let mut tx = pool.begin().await.unwrap();
    
    query("DELETE FROM trades").execute(&mut *tx).await.unwrap();
    query("DELETE FROM orders").execute(&mut *tx).await.unwrap();
    query("DELETE FROM user_balances").execute(&mut *tx).await.unwrap();
    
    tx.commit().await.unwrap();
}

/// 테스트용 잔고 설정
/// 
/// 여러 테스트 사용자를 생성하고 초기 잔고를 설정합니다.
/// - 유저 100명 생성
/// - 각 유저에게 SOL, USDT 잔고 부여
/// - 실제 테스트는 TEST_USER_ID (1번 유저)만 사용
async fn setup_test_balances(db: &Database) {
    use sqlx::query;
    
    let pool = db.pool();
    let mut tx = pool.begin().await.unwrap();
    
    // 유저 100명 생성 및 잔고 설정
    for user_id in 1..=NUM_TEST_USERS {
        // 테스트 사용자 생성 (없으면 생성)
        query(
            r#"
            INSERT INTO users (id, email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(user_id as i64)
        .bind(format!("test_user_{}@example.com", user_id))
        .bind("dummy_hash")
        .execute(&mut *tx)
        .await
        .unwrap();
        
        // SOL 잔고
        query(
            r#"
            INSERT INTO user_balances (user_id, mint_address, available, locked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (user_id, mint_address) DO UPDATE
            SET available = $3, locked = $4, updated_at = NOW()
            "#,
        )
        .bind(user_id as i64)
        .bind("SOL")
        .bind(initial_sol_balance())
        .bind(Decimal::ZERO)
        .execute(&mut *tx)
        .await
        .unwrap();
        
        // USDT 잔고
        query(
            r#"
            INSERT INTO user_balances (user_id, mint_address, available, locked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (user_id, mint_address) DO UPDATE
            SET available = $3, locked = $4, updated_at = NOW()
            "#,
        )
        .bind(user_id as i64)
        .bind("USDT")
        .bind(initial_usdt_balance())
        .bind(Decimal::ZERO)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    
    tx.commit().await.unwrap();
}

/// 오더북 쌓기 (사전 셋업)
/// 
/// 테스트를 위해 오더북에 주문들을 미리 채워놓습니다.
/// 거래가 일어나지 않도록 매수 주문과 매도 주문의 가격을 겹치지 않게 설정합니다.
/// 
/// 전략:
/// - 유저 2~50: 매수 주문 (가격 90~100 USDT, 매수 가격 < 매도 가격)
/// - 유저 51~100: 매도 주문 (가격 101~110 USDT, 매도 가격 > 매수 가격)
/// 
/// 이렇게 하면 매칭되지 않고 오더북에만 쌓입니다.
async fn setup_orderbook(engine: &mut HighPerformanceEngine) {
    use chrono::Utc;
    
    // 매수 주문: 유저 2~50번 (가격 90~100 USDT)
    // 가격이 낮을수록 매수 의향이 높음 (높은 가격에 사고 싶지 않음)
    // 49명의 유저를 11개 가격 레벨(90~100)에 분배
    for i in 2..=50 {
        let user_id = i as u64;
        // 90부터 100까지 균등 분배 (11개 레벨)
        let price_level = (i - 2) % 11; // 0~10
        let price = Decimal::new(90 + price_level, 0); // 90, 91, 92... 100
        let amount = Decimal::new(100, 0); // 100 SOL
        
        let order = OrderEntry {
            id: user_id * 1000, // 고유 ID 생성
            user_id,
            order_type: "buy".to_string(),
            order_side: "limit".to_string(),
            base_mint: "SOL".to_string(),
            quote_mint: "USDT".to_string(),
            price: Some(price),
            amount,
            filled_amount: Decimal::ZERO,
            remaining_amount: amount,
            quote_amount: None,
            remaining_quote_amount: None,
            created_at: Utc::now(),
        };
        
        engine.submit_order(order).await
            .expect(&format!("Failed to submit buy order for user {}", user_id));
    }
    
    // 매도 주문: 유저 51~100번 (가격 101~110 USDT)
    // 가격이 높을수록 매도 의향이 높음 (낮은 가격에 팔고 싶지 않음)
    // 50명의 유저를 10개 가격 레벨(101~110)에 분배
    for i in 51..=100 {
        let user_id = i as u64;
        // 101부터 110까지 균등 분배 (10개 레벨)
        let price_level = (i - 51) % 10; // 0~9
        let price = Decimal::new(101 + price_level, 0); // 101, 102, 103... 110
        let amount = Decimal::new(100, 0); // 100 SOL
        
        let order = OrderEntry {
            id: user_id * 1000, // 고유 ID 생성
            user_id,
            order_type: "sell".to_string(),
            order_side: "limit".to_string(),
            base_mint: "SOL".to_string(),
            quote_mint: "USDT".to_string(),
            price: Some(price),
            amount,
            filled_amount: Decimal::ZERO,
            remaining_amount: amount,
            quote_amount: None,
            remaining_quote_amount: None,
            created_at: Utc::now(),
        };
        
        engine.submit_order(order).await
            .expect(&format!("Failed to submit sell order for user {}", user_id));
    }
    
    println!("Orderbook populated: {} buy orders (90-100 USDT) + {} sell orders (101-110 USDT)", 
             49, 50);
}

/// 테스트: 엔진 시작 및 중지
/// 
/// 엔진이 정상적으로 시작되고 중지되는지 확인합니다.
#[tokio::test]
async fn test_engine_start_stop() {
    let (mut engine, db) = setup_test().await;
    
    // 엔진이 정상적으로 시작되었는지 확인
    // (에러가 없으면 성공)
    
    teardown_test(&mut engine, &db).await;
}

/// 테스트: 잔고 조회
/// 
/// 엔진 시작 후 잔고가 제대로 로드되었는지 확인합니다.
#[tokio::test]
async fn test_balance_loaded() {
    let (mut engine, db) = setup_test().await;
    
    // SOL 잔고 조회
    let (sol_available, sol_locked) = engine.get_balance(TEST_USER_ID, "SOL").await
        .expect("Failed to get SOL balance");
    assert_eq!(sol_available, initial_sol_balance());
    assert_eq!(sol_locked, Decimal::ZERO);
    
    // USDT 잔고 조회
    let (usdt_available, usdt_locked) = engine.get_balance(TEST_USER_ID, "USDT").await
        .expect("Failed to get USDT balance");
    assert_eq!(usdt_available, initial_usdt_balance());
    assert_eq!(usdt_locked, Decimal::ZERO);
    
    teardown_test(&mut engine, &db).await;
}

/// 테스트: 모든 유저 잔고 로드 확인
/// 
/// 100명의 유저 잔고가 모두 엔진에 로드되었는지 확인합니다.
#[tokio::test]
async fn test_all_users_balances_loaded() {
    let (mut engine, db) = setup_test().await;
    
    // 1번부터 100번까지 모든 유저의 잔고 확인
    for user_id in 1..=NUM_TEST_USERS {
        // SOL 잔고 조회
        let (sol_available, sol_locked) = engine.get_balance(user_id, "SOL").await
            .expect(&format!("Failed to get SOL balance for user {}", user_id));
        assert_eq!(sol_available, initial_sol_balance(), "User {} SOL balance mismatch", user_id);
        assert_eq!(sol_locked, Decimal::ZERO, "User {} SOL locked should be 0", user_id);
        
        // USDT 잔고 조회
        let (usdt_available, usdt_locked) = engine.get_balance(user_id, "USDT").await
            .expect(&format!("Failed to get USDT balance for user {}", user_id));
        assert_eq!(usdt_available, initial_usdt_balance(), "User {} USDT balance mismatch", user_id);
        assert_eq!(usdt_locked, Decimal::ZERO, "User {} USDT locked should be 0", user_id);
    }
    
    println!("All {} users' balances loaded successfully", NUM_TEST_USERS);
    
    teardown_test(&mut engine, &db).await;
}

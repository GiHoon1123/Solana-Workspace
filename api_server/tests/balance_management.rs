// =====================================================
// 잔고 관리 통합 테스트
// =====================================================

mod common;
use common::*;
use rust_decimal::Decimal;
use api_server::domains::cex::engine::Engine;

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
/// 테스트용 계정(1~10번)은 오더북에 주문이 없으므로 잔고가 락되지 않습니다.
/// 오더북용 계정(11~100번)은 주문으로 인해 잔고가 락되지만, 초기 잔고는 정상적으로 로드되었는지 확인합니다.
#[tokio::test]
async fn test_all_users_balances_loaded() {
    let (mut engine, db) = setup_test().await;  // 오더북 포함 (11~100번만 사용)
    
    // 테스트용 계정(1~10번): 오더북에 주문이 없으므로 잔고가 락되지 않음
    for user_id in 1..=10 {
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
    
    // 오더북용 계정(11~100번): 주문으로 인해 잔고가 락되지만, 초기 잔고는 정상적으로 로드되었는지 확인
    // (락된 잔고는 주문 가격과 수량에 따라 다르므로, available + locked = 초기 잔고인지만 확인)
    for user_id in 11..=NUM_TEST_USERS {
        // SOL 잔고 조회
        let (sol_available, sol_locked) = engine.get_balance(user_id, "SOL").await
            .expect(&format!("Failed to get SOL balance for user {}", user_id));
        // available + locked = 초기 잔고 (주문으로 인해 일부가 락됨)
        assert_eq!(sol_available + sol_locked, initial_sol_balance(), 
                   "User {} SOL total (available + locked) should equal initial balance", user_id);
        
        // USDT 잔고 조회
        let (usdt_available, usdt_locked) = engine.get_balance(user_id, "USDT").await
            .expect(&format!("Failed to get USDT balance for user {}", user_id));
        // available + locked = 초기 잔고 (주문으로 인해 일부가 락됨)
        assert_eq!(usdt_available + usdt_locked, initial_usdt_balance(), 
                   "User {} USDT total (available + locked) should equal initial balance", user_id);
    }
    
    println!("✅ All {} users' balances loaded successfully", NUM_TEST_USERS);
    
    teardown_test(&mut engine, &db).await;
}


// =====================================================
// Threads - 스레드 루프 함수들
// =====================================================
// 역할: 엔진 스레드와 WAL 스레드의 메인 루프 구현
//
// 구조:
// - engine_thread_loop(): 주문 처리 루프 (Core 0)
// - wal_thread_loop(): WAL 쓰기 루프 (Core 1)
// =====================================================

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context};
use crossbeam::channel::Receiver;
use parking_lot::{RwLock, Mutex};
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::domains::cex::engine::types::{TradingPair, OrderEntry, MatchResult};
use crate::domains::cex::engine::orderbook::OrderBook;
use crate::domains::cex::engine::matcher::Matcher;
use crate::domains::cex::engine::executor::Executor;
use crate::domains::cex::engine::wal::{WalEntry, WalWriter};

use super::commands::OrderCommand;
use super::config::CoreConfig;

// =====================================================
// 엔진 스레드 루프
// =====================================================
// 역할: 모든 주문을 순차적으로 처리하는 싱글 스레드 루프
//
// 처리 과정:
// 1. 코어 고정 (Core 0)
// 2. 실시간 스케줄링 (SCHED_FIFO, 우선순위 99)
// 3. 주문 명령 수신 루프
// 4. 각 명령 처리 (SubmitOrder, CancelOrder 등)
// 5. 결과를 oneshot 채널로 반환
// =====================================================

/// 엔진 스레드 메인 루프
/// 
/// # Arguments
/// * `order_rx` - 주문 명령 수신 채널
/// * `wal_tx` - WAL 메시지 전송 채널
/// * `orderbooks` - 거래쌍별 오더북 (공유)
/// * `matcher` - 매칭 엔진 (공유)
/// * `executor` - 체결 실행 엔진 (공유)
/// * `running` - 실행 중 여부 플래그
/// 
/// # 처리 흐름
/// ```
/// loop {
///     order_rx.recv() → OrderCommand
///         ↓
///     match OrderCommand {
///         SubmitOrder → 
///             1. WAL 메시지 발행
///             2. OrderBook에 추가
///             3. Matcher로 매칭
///             4. Executor로 체결
///             5. 결과 반환
///         CancelOrder → ...
///         GetOrderbook → ...
///         ...
///     }
/// }
/// ```
/// 
/// # 성능
/// - 주문 처리: < 0.5ms (평균)
/// - 체결 처리: < 0.2ms (평균)
/// - TPS: 50,000+ orders/sec
pub fn engine_thread_loop(
    order_rx: Receiver<OrderCommand>,
    wal_tx: crossbeam::channel::Sender<WalEntry>,
    orderbooks: Arc<RwLock<HashMap<TradingPair, OrderBook>>>,
    matcher: Arc<Matcher>,
    executor: Arc<Mutex<Executor>>,
    running: Arc<std::sync::atomic::AtomicBool>,
) {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. 코어 고정 (Core 0)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    let config = CoreConfig::from_env();
    CoreConfig::set_core(Some(config.engine_core));
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 2. 실시간 스케줄링 설정 (우선순위 99)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    CoreConfig::set_realtime_scheduling(99);
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 2. 메인 루프
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    loop {
        // running 플래그 확인
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        
        // 주문 명령 수신 (블로킹)
        match order_rx.recv() {
            Ok(cmd) => {
                // 명령 처리
                match cmd {
                    OrderCommand::SubmitOrder { order, response } => {
                        handle_submit_order(
                            order,
                            response,
                            &wal_tx,
                            &orderbooks,
                            &matcher,
                            &executor,
                        );
                    }
                    OrderCommand::CancelOrder { order_id, user_id, trading_pair, response } => {
                        handle_cancel_order(
                            order_id,
                            user_id,
                            trading_pair,
                            response,
                            &wal_tx,
                            &orderbooks,
                            &executor,
                        );
                    }
                    OrderCommand::GetOrderbook { trading_pair, depth, response } => {
                        handle_get_orderbook(
                            trading_pair,
                            depth,
                            response,
                            &orderbooks,
                        );
                    }
                    OrderCommand::GetBalance { user_id, mint, response } => {
                        handle_get_balance(
                            user_id,
                            mint,
                            response,
                            &executor,
                        );
                    }
                    OrderCommand::LockBalance { user_id, mint, amount, response } => {
                        handle_lock_balance(
                            user_id,
                            mint,
                            amount,
                            response,
                            &wal_tx,
                            &executor,
                        );
                    }
                    OrderCommand::UnlockBalance { user_id, mint, amount, response } => {
                        handle_unlock_balance(
                            user_id,
                            mint,
                            amount,
                            response,
                            &wal_tx,
                            &executor,
                        );
                    }
                }
            }
            Err(_) => {
                // 채널이 닫힘 (정상 종료)
                break;
            }
        }
    }
}

// =====================================================
// 명령 처리 핸들러들
// =====================================================

/// SubmitOrder 명령 처리
/// 
/// # 처리 과정
/// 1. WAL 메시지 발행 (OrderCreated)
/// 2. OrderBook에 추가
/// 3. Matcher로 매칭 시도
/// 4. 체결된 경우 Executor로 처리
/// 5. MatchResult 목록을 response로 전송
fn handle_submit_order(
    mut order: OrderEntry,
    response: tokio::sync::oneshot::Sender<Result<Vec<MatchResult>>>,
    wal_tx: &crossbeam::channel::Sender<WalEntry>,
    orderbooks: &Arc<RwLock<HashMap<TradingPair, OrderBook>>>,
    matcher: &Arc<Matcher>,
    executor: &Arc<Mutex<Executor>>,
) {
    // 1. TradingPair 찾기
    let pair = TradingPair::new(order.base_mint.clone(), order.quote_mint.clone());
    
    // 2. 잔고 잠금 (주문 제출 전에 잠금)
    {
        let mut executor_guard = executor.lock();
        let (lock_mint, lock_amount) = if order.order_type == "buy" {
            // 매수: quote_mint 잠금
            // 지정가: price * amount
            // 시장가: quote_amount
            let amount = if order.order_side == "market" {
                // 시장가 매수: quote_amount 사용
                order.quote_amount.unwrap_or(rust_decimal::Decimal::ZERO)
            } else {
                // 지정가 매수: price * amount
                order.price.unwrap_or(rust_decimal::Decimal::ZERO) * order.amount
            };
            (&order.quote_mint, amount)
        } else {
            // 매도: base_mint 잠금 (amount만큼)
            (&order.base_mint, order.amount)
        };
        
        if let Err(e) = executor_guard.lock_balance_for_order(order.id, order.user_id, lock_mint, lock_amount) {
            // 에러 상세 정보 출력
            if let Some(balance) = executor_guard.balance_cache().get_balance(order.user_id, lock_mint) {
                eprintln!(
                    "Lock failed: user_id={}, mint={}, available={}, locked={}, required={}, error={}",
                    order.user_id, lock_mint, balance.available, balance.locked, lock_amount, e
                );
            } else {
                eprintln!(
                    "Lock failed: user_id={}, mint={}, required={}, error={} (balance not found in cache)",
                    order.user_id, lock_mint, lock_amount, e
                );
            }
            let _ = response.send(Err(anyhow::anyhow!("Failed to lock balance: {}", e)));
            return;
        }
    }
    
    // 3. WAL 메시지 발행 (OrderCreated) - 잔고 잠금 후!
    let wal_entry = WalEntry::OrderCreated {
        order_id: order.id,
        user_id: order.user_id,
        order_type: order.order_type.clone(),
        base_mint: order.base_mint.clone(),
        quote_mint: order.quote_mint.clone(),
        price: order.price.map(|p| p.to_string()),
        amount: order.amount.to_string(),
        timestamp: order.created_at.timestamp_millis(),
    };
    let _ = wal_tx.send(wal_entry);
    
    // 4. 시장가 주문 여부 및 초기 잔고 잠금 정보 저장 (order 이동 전)
    let is_market_order = order.order_side == "market";
    let initial_quote_amount = order.quote_amount;
    let initial_amount = order.amount;
    
    // 5. OrderBook 가져오기 및 매칭 (락 안에서 수행)
    let (matches, order_after_match) = {
        let mut orderbooks_guard = orderbooks.write();
        let orderbook = orderbooks_guard.entry(pair.clone()).or_insert_with(|| OrderBook::new(pair.clone()));
        
        // 6. Matcher로 매칭 시도 (먼저 매칭 시도)
        let matches = matcher.match_order(&mut order, orderbook);
        
        // 7. 매칭 후 남은 주문이 있으면 OrderBook에 추가
        // 시장가 주문은 완전히 체결되지 않으면 오더북에 추가하지 않음 (시장가 주문은 즉시 체결되어야 함)
        // 지정가 주문은 부분 체결 후 남은 수량이 있으면 오더북에 추가
        if order.order_side == "limit" {
            // 지정가 주문: 남은 수량이 있으면 오더북에 추가
            let has_remaining = if let Some(remaining_quote) = order.remaining_quote_amount {
                remaining_quote > Decimal::ZERO
            } else {
                order.remaining_amount > Decimal::ZERO
            };
            
            if has_remaining {
                orderbook.add_order(order.clone()); // order는 나중에 사용하므로 클론
            }
        }
        // 시장가 주문은 완전히 체결되지 않으면 오더북에 추가하지 않음
        
        // order 상태 저장 (매칭 후)
        let order_after_match = order.clone();
        (matches, order_after_match)
    };
    
    // 8. 시장가 주문 완전 체결 확인 및 처리
    let is_fully_filled = if is_market_order {
        // 시장가 주문: 완전히 체결되었는지 확인
        if let Some(remaining_quote) = order_after_match.remaining_quote_amount {
            // 시장가 매수 (금액 기반): remaining_quote_amount가 0이면 완전 체결
            remaining_quote <= Decimal::ZERO
        } else {
            // 시장가 매도 (수량 기반): remaining_amount가 0이면 완전 체결
            order_after_match.remaining_amount == Decimal::ZERO
        }
    } else {
        // 지정가 주문: 항상 성공 (부분 체결되어도 오더북에 추가됨)
        true
    };
    
    // 시장가 주문이 완전히 체결되지 않았으면 잔고 잠금 해제 및 에러 반환
    if is_market_order && !is_fully_filled {
        // 부분 체결이 있으면 먼저 처리
        {
            let mut executor_guard = executor.lock();
            for match_result in &matches {
                if let Err(e) = executor_guard.execute_trade(match_result) {
                    eprintln!("Failed to execute trade: {}", e);
                }
            }
        }
        
        // 남은 잔고 잠금 해제
        {
            let mut executor_guard = executor.lock();
            let (unlock_mint, unlock_amount) = if order_after_match.order_type == "buy" {
                // 시장가 매수: 남은 quote_amount만큼 USDT 잠금 해제
                let remaining = order_after_match.remaining_quote_amount.unwrap_or(Decimal::ZERO);
                (&order_after_match.quote_mint, remaining)
            } else {
                // 시장가 매도: 남은 amount만큼 SOL 등 잠금 해제
                (&order_after_match.base_mint, order_after_match.remaining_amount)
            };
            
            if unlock_amount > Decimal::ZERO {
                if let Err(e) = executor_guard.unlock_balance_for_cancel(
                    order_after_match.id,
                    order_after_match.user_id,
                    unlock_mint,
                    unlock_amount,
                ) {
                    eprintln!("Failed to unlock balance: {}", e);
                }
            }
        }
        
        // 에러 반환
        let _ = response.send(Err(anyhow::anyhow!(
            "Market order partially filled or not filled at all. Matches: {}, Remaining: {}",
            matches.len(),
            if order_after_match.order_type == "buy" {
                order_after_match.remaining_quote_amount.unwrap_or(Decimal::ZERO)
            } else {
                order_after_match.remaining_amount
            }
        )));
        return;
    }
    
    // 시장가 주문이 매칭되지 않았으면 (matches가 비어있음) 잔고 잠금 해제 및 에러 반환
    if is_market_order && matches.is_empty() {
        // 잔고 잠금 해제
        {
            let mut executor_guard = executor.lock();
            let (unlock_mint, unlock_amount) = if order_after_match.order_type == "buy" {
                // 시장가 매수: 전체 quote_amount만큼 USDT 잠금 해제
                let amount = initial_quote_amount.unwrap_or(Decimal::ZERO);
                (&order_after_match.quote_mint, amount)
            } else {
                // 시장가 매도: 전체 amount만큼 SOL 등 잠금 해제
                (&order_after_match.base_mint, initial_amount)
            };
            
            if unlock_amount > Decimal::ZERO {
                if let Err(e) = executor_guard.unlock_balance_for_cancel(
                    order_after_match.id,
                    order_after_match.user_id,
                    unlock_mint,
                    unlock_amount,
                ) {
                    eprintln!("Failed to unlock balance: {}", e);
                }
            }
        }
        
        // 에러 반환
        let _ = response.send(Err(anyhow::anyhow!(
            "Market order cannot be filled: no matching orders in orderbook"
        )));
        return;
    }
    
    // 8. 체결 처리 (정상 케이스: 지정가 주문 또는 완전히 체결된 시장가 주문)
    {
        let mut executor_guard = executor.lock();
        for match_result in &matches {
            if let Err(e) = executor_guard.execute_trade(match_result) {
                // 에러 발생 시 로그 기록
                eprintln!("Failed to execute trade: {}", e);
            }
        }
    }
    
    // 9. 결과 반환
    let _ = response.send(Ok(matches));
}

/// CancelOrder 명령 처리
/// 
/// # 처리 과정
/// 1. OrderBook에서 주문 찾기
/// 2. 권한 확인 (user_id 일치)
/// 3. WAL 메시지 발행 (OrderCancelled)
/// 4. OrderBook에서 제거
/// 5. 잔고 잠금 해제 (remaining_amount만큼)
/// 6. 취소된 주문 반환
fn handle_cancel_order(
    order_id: u64,
    user_id: u64,
    trading_pair: TradingPair,
    response: tokio::sync::oneshot::Sender<Result<OrderEntry>>,
    wal_tx: &crossbeam::channel::Sender<WalEntry>,
    orderbooks: &Arc<RwLock<HashMap<TradingPair, OrderBook>>>,
    executor: &Arc<Mutex<Executor>>,
) {
    // 1. OrderBook에서 주문 찾기
    let mut orderbooks_guard = orderbooks.write();
    let orderbook = match orderbooks_guard.get_mut(&trading_pair) {
        Some(ob) => ob,
        None => {
            let _ = response.send(Err(anyhow::anyhow!("OrderBook not found for trading pair")));
            return;
        }
    };
    
    // 2. 주문 찾기 (매수/매도 양쪽 모두 확인)
    let mut found_order: Option<OrderEntry> = None;
    let mut found_price: Option<rust_decimal::Decimal> = None;
    let mut is_buy = false;
    
    // 매수 호가에서 찾기
    for (price, orders) in orderbook.buy_orders.orders.iter_mut() {
        if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
            if let Some(order) = orders.remove(pos) {
                // 권한 확인
                if order.user_id == user_id {
                    found_order = Some(order.clone());
                    found_price = Some(*price);
                    is_buy = true;
                    if orders.is_empty() {
                        // 빈 VecDeque는 나중에 제거
                    }
                    break;
                } else {
                    // 권한 없음 - 다시 추가
                    orders.insert(pos, order);
                    let _ = response.send(Err(anyhow::anyhow!("Unauthorized: You don't own this order")));
                    return;
                }
            }
        }
    }
    
    // 매도 호가에서 찾기
    if found_order.is_none() {
        for (price, orders) in orderbook.sell_orders.orders.iter_mut() {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                if let Some(order) = orders.remove(pos) {
                    // 권한 확인
                    if order.user_id == user_id {
                        found_order = Some(order.clone());
                        found_price = Some(*price);
                        is_buy = false;
                        if orders.is_empty() {
                            // 빈 VecDeque는 나중에 제거
                        }
                        break;
                    } else {
                        // 권한 없음 - 다시 추가
                        orders.insert(pos, order);
                        let _ = response.send(Err(anyhow::anyhow!("Unauthorized: You don't own this order")));
                        return;
                    }
                }
            }
        }
    }
    
    // 빈 VecDeque 제거
    if let Some(price) = found_price {
        if is_buy {
            if let Some(orders) = orderbook.buy_orders.orders.get(&price) {
                if orders.is_empty() {
                    orderbook.buy_orders.orders.remove(&price);
                }
            }
        } else {
            if let Some(orders) = orderbook.sell_orders.orders.get(&price) {
                if orders.is_empty() {
                    orderbook.sell_orders.orders.remove(&price);
                }
            }
        }
    }
    
    let order_type = found_order.as_ref().map(|o| o.order_type.clone());
    
    // 주문을 찾지 못함
    let order = match found_order {
        Some(o) => o,
        None => {
            let _ = response.send(Err(anyhow::anyhow!("Order not found")));
            return;
        }
    };
    
    // 3. WAL 메시지 발행 (OrderCancelled)
    let wal_entry = WalEntry::OrderCancelled {
        order_id,
        user_id,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    let _ = wal_tx.send(wal_entry);
    
    // 4. 잔고 잠금 해제 (remaining_amount만큼)
    let order_type_str = order_type.as_deref().unwrap_or("buy");
    let unlock_mint = if order_type_str == "buy" {
        &order.quote_mint  // 매수: USDT 잠금 해제
    } else {
        &order.base_mint   // 매도: SOL 등 잠금 해제
    };
    
    // 잠금 해제할 금액 계산
    let unlock_amount = if order_type_str == "buy" {
        // 매수: price * remaining_amount
        order.price.unwrap_or(rust_decimal::Decimal::ZERO) * order.remaining_amount
    } else {
        // 매도: remaining_amount
        order.remaining_amount
    };
    
    // Executor로 잔고 잠금 해제
    {
        let mut executor_guard = executor.lock();
        if let Err(e) = executor_guard.unlock_balance_for_cancel(order_id, user_id, unlock_mint, unlock_amount) {
            let _ = response.send(Err(anyhow::anyhow!("Failed to unlock balance: {}", e)));
            return;
        }
    }
    
    // 5. 취소된 주문 반환
    let _ = response.send(Ok(order));
}

/// GetOrderbook 명령 처리
/// 
/// # 처리 과정
/// 1. OrderBook 찾기
/// 2. 매수 주문 목록 수집 (depth만큼)
/// 3. 매도 주문 목록 수집 (depth만큼)
/// 4. 결과 반환
/// 
/// # Note
/// depth가 None이면 전체 주문 반환 (주의: 많은 주문이 있으면 느릴 수 있음)
fn handle_get_orderbook(
    trading_pair: TradingPair,
    depth: Option<usize>,
    response: tokio::sync::oneshot::Sender<Result<(Vec<OrderEntry>, Vec<OrderEntry>)>>,
    orderbooks: &Arc<RwLock<HashMap<TradingPair, OrderBook>>>,
) {
    // 1. OrderBook 찾기
    let orderbooks_guard = orderbooks.read();
    let orderbook = match orderbooks_guard.get(&trading_pair) {
        Some(ob) => ob,
        None => {
            // OrderBook이 없으면 빈 목록 반환
            let _ = response.send(Ok((Vec::new(), Vec::new())));
            return;
        }
    };
    
    // 2. 매수 주문 목록 수집
    let mut buy_orders = Vec::new();
    for (_, orders) in orderbook.buy_orders.orders.iter().rev() {
        for order in orders.iter() {
            buy_orders.push(order.clone());
            if let Some(d) = depth {
                if buy_orders.len() >= d {
                    break;
                }
            }
        }
        if let Some(d) = depth {
            if buy_orders.len() >= d {
                break;
            }
        }
    }
    
    // 3. 매도 주문 목록 수집
    let mut sell_orders = Vec::new();
    for (_, orders) in orderbook.sell_orders.orders.iter() {
        for order in orders.iter() {
            sell_orders.push(order.clone());
            if let Some(d) = depth {
                if sell_orders.len() >= d {
                    break;
                }
            }
        }
        if let Some(d) = depth {
            if sell_orders.len() >= d {
                break;
            }
        }
    }
    
    // 4. 결과 반환
    let _ = response.send(Ok((buy_orders, sell_orders)));
}

/// GetBalance 명령 처리
fn handle_get_balance(
    user_id: u64,
    mint: String,
    response: tokio::sync::oneshot::Sender<Result<(rust_decimal::Decimal, rust_decimal::Decimal)>>,
    executor: &Arc<Mutex<Executor>>,
) {
    let executor = executor.lock();
    let balance_cache = executor.balance_cache();
    
    match balance_cache.get_balance(user_id, &mint) {
        Some(balance) => {
            let _ = response.send(Ok((balance.available, balance.locked)));
        }
        None => {
            let _ = response.send(Ok((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO)));
        }
    }
}

/// LockBalance 명령 처리
fn handle_lock_balance(
    user_id: u64,
    mint: String,
    amount: rust_decimal::Decimal,
    response: tokio::sync::oneshot::Sender<Result<()>>,
    wal_tx: &crossbeam::channel::Sender<WalEntry>,
    executor: &Arc<Mutex<Executor>>,
) {
    let mut executor = executor.lock();
    
    // WAL 메시지 발행
    let wal_entry = WalEntry::BalanceLocked {
        user_id,
        mint: mint.clone(),
        amount: amount.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    let _ = wal_tx.send(wal_entry);
    
    // 잔고 잠금
    match executor.lock_balance_for_order(0, user_id, &mint, amount) {
        Ok(()) => {
            let _ = response.send(Ok(()));
        }
        Err(e) => {
            let _ = response.send(Err(e));
        }
    }
}

/// UnlockBalance 명령 처리
fn handle_unlock_balance(
    user_id: u64,
    mint: String,
    amount: rust_decimal::Decimal,
    response: tokio::sync::oneshot::Sender<Result<()>>,
    wal_tx: &crossbeam::channel::Sender<WalEntry>,
    executor: &Arc<Mutex<Executor>>,
) {
    let mut executor = executor.lock();
    
    // WAL 메시지 발행
    let wal_entry = WalEntry::OrderCancelled {
        order_id: 0,  // TODO: 실제 order_id 전달
        user_id,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    let _ = wal_tx.send(wal_entry);
    
    // 잔고 잠금 해제
    match executor.unlock_balance_for_cancel(0, user_id, &mint, amount) {
        Ok(()) => {
            let _ = response.send(Ok(()));
        }
        Err(e) => {
            let _ = response.send(Err(e));
        }
    }
}

// =====================================================
// WAL 스레드 루프
// =====================================================
// 역할: WAL 메시지를 받아서 디스크에 순차 쓰기
//
// 처리 과정:
// 1. 코어 고정 (Core 1)
// 2. WalWriter 생성
// 3. WAL 메시지 수신 루프
// 4. WalWriter::append() 호출
// 5. 주기적 fsync()
// =====================================================

/// WAL 스레드 메인 루프
/// 
/// # Arguments
/// * `wal_rx` - WAL 메시지 수신 채널
/// * `wal_dir` - WAL 디렉토리 경로
/// 
/// # 처리 흐름
/// ```
/// loop {
///     wal_rx.recv() → WalEntry
///         ↓
///     WalWriter::append(entry)
///         ↓
///     (10개마다) fsync()
/// }
/// ```
pub fn wal_thread_loop(
    wal_rx: Receiver<WalEntry>,
    wal_dir: std::path::PathBuf,
) {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. 코어 고정 (Core 1)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    let config = CoreConfig::from_env();
    CoreConfig::set_core(Some(config.wal_core));
    
    // WAL 스레드는 실시간 스케줄링 불필요 (I/O 바운드)
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 2. WalWriter 생성
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    let mut wal_writer = match WalWriter::new(&wal_dir, 10) {
        Ok(writer) => writer,
        Err(e) => {
            eprintln!("Failed to create WalWriter: {}", e);
            return;
        }
    };
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 3. 메인 루프
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    loop {
        match wal_rx.recv() {
            Ok(entry) => {
                // WAL 파일에 쓰기
                if let Err(e) = wal_writer.append(&entry) {
                    eprintln!("Failed to write to WAL: {}", e);
                }
            }
            Err(_) => {
                // 채널이 닫힘 (정상 종료)
                // 마지막 동기화
                let _ = wal_writer.sync();
                break;
            }
        }
    }
}

// =====================================================
// DB Writer 스레드 루프
// =====================================================
// 역할: 메모리에서 DB 명령을 받아서 배치로 DB에 저장
//
// 처리 과정:
// 1. 코어 고정 (Core 2, dev 환경만)
// 2. DB 명령 수신 루프
// 3. 배치로 모으기 (10ms 또는 100개)
// 4. DB에 배치 쓰기 (트랜잭션)
// =====================================================

/// DB Writer 스레드 메인 루프
/// 
/// # Arguments
/// * `db_rx` - DB 명령 수신 채널
/// * `db_pool` - 데이터베이스 연결 풀
/// 
/// # 처리 흐름
/// ```
/// loop {
///     db_rx.recv() → DbCommand
///         ↓
///     batch.push(cmd)
///         ↓
///     (10ms 또는 100개마다)
///         ↓
///     db.execute_batch(&batch)
/// }
/// ```
/// 
/// # 배치 전략
/// - 시간 기반: 10ms마다 배치 쓰기
/// - 크기 기반: 100개 모이면 즉시 쓰기
/// - 트랜잭션: 여러 작업을 하나의 트랜잭션으로 묶기
pub fn db_writer_thread_loop(
    db_rx: Receiver<super::db_commands::DbCommand>,
    db_pool: PgPool,
) {
    use super::db_commands::DbCommand;
    use std::time::{Duration, Instant};
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. 코어 고정 (Core 2, dev 환경만)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    let config = CoreConfig::from_env();
    if let Some(core) = config.db_writer_core {
        CoreConfig::set_core(Some(core));
    }
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 2. 배치 변수 초기화
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    let mut batch = Vec::new();
    let batch_size_limit = 100;
    let batch_time_limit = Duration::from_millis(10);
    let mut last_flush = Instant::now();
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 3. 메인 루프 (Tokio 런타임 필요)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    // Tokio 런타임 생성 (DB 작업은 async이므로)
    let rt = tokio::runtime::Runtime::new()
        .expect("Failed to create Tokio runtime for DB Writer");
    
    rt.block_on(async {
        loop {
            // 타임아웃 설정 (10ms)
            let timeout = batch_time_limit.saturating_sub(last_flush.elapsed());
            
            match db_rx.recv_timeout(timeout) {
                Ok(cmd) => {
                    // 명령 수신
                    batch.push(cmd);
                    
                    // 크기 기반 배치 쓰기 (100개 모이면)
                    if batch.len() >= batch_size_limit {
                        if let Err(e) = flush_batch(&mut batch, &db_pool).await {
                            eprintln!("Failed to flush DB batch: {}", e);
                        }
                        last_flush = Instant::now();
                    }
                }
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                    // 시간 기반 배치 쓰기 (10ms 경과)
                    if !batch.is_empty() {
                        if let Err(e) = flush_batch(&mut batch, &db_pool).await {
                            eprintln!("Failed to flush DB batch: {}", e);
                        }
                        last_flush = Instant::now();
                    }
                }
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    // 채널이 닫힘 (정상 종료)
                    // 마지막 배치 쓰기
                    if !batch.is_empty() {
                        let _ = flush_batch(&mut batch, &db_pool).await;
                    }
                    break;
                }
            }
        }
    });
}

/// 배치를 DB에 쓰기 (async)
/// 
/// # Arguments
/// * `batch` - DB 명령 배치
/// * `db_pool` - 데이터베이스 연결 풀
/// 
/// # 처리 과정
/// 1. 트랜잭션 시작
/// 2. 각 명령 처리
/// 3. 커밋
async fn flush_batch(
    batch: &mut Vec<super::db_commands::DbCommand>,
    db_pool: &PgPool,
) -> Result<()> {
    use super::db_commands::DbCommand;
    
    if batch.is_empty() {
        return Ok(());
    }
    
    // 트랜잭션 시작
    let mut tx = db_pool.begin().await
        .context("Failed to begin transaction")?;
    
    // 각 명령 처리
    for cmd in batch.drain(..) {
        match cmd {
            DbCommand::InsertOrder {
                order_id,
                user_id,
                order_type,
                order_side,
                base_mint,
                quote_mint,
                price,
                amount,
                created_at,
            } => {
                sqlx::query(
                    r#"
                    INSERT INTO orders (
                        id, user_id, order_type, order_side, base_mint, quote_mint,
                        price, amount, filled_amount, status, created_at, updated_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT (id) DO NOTHING
                    "#
                )
                .bind(order_id as i64)
                .bind(user_id as i64)
                .bind(&order_type)
                .bind(&order_side)
                .bind(&base_mint)
                .bind(&quote_mint)
                .bind(&price)
                .bind(&amount)
                .bind(rust_decimal::Decimal::ZERO)  // filled_amount
                .bind("pending")  // status
                .bind(created_at)
                .bind(created_at)
                .execute(&mut *tx)
                .await
                .context("Failed to insert order")?;
            }
            
            DbCommand::UpdateOrderStatus {
                order_id,
                status,
                filled_amount,
            } => {
                sqlx::query(
                    r#"
                    UPDATE orders
                    SET status = $1, filled_amount = $2, updated_at = $3
                    WHERE id = $4
                    "#
                )
                .bind(&status)
                .bind(&filled_amount)
                .bind(chrono::Utc::now())
                .bind(order_id as i64)
                .execute(&mut *tx)
                .await
                .context("Failed to update order status")?;
            }
            
            DbCommand::InsertTrade {
                trade_id,
                buy_order_id,
                sell_order_id,
                buyer_id,
                seller_id,
                price,
                amount,
                base_mint,
                quote_mint,
                timestamp,
            } => {
                sqlx::query(
                    r#"
                    INSERT INTO trades (
                        id, buy_order_id, sell_order_id, buyer_id, seller_id,
                        price, amount, base_mint, quote_mint, created_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT (id) DO NOTHING
                    "#
                )
                .bind(trade_id as i64)
                .bind(buy_order_id as i64)
                .bind(sell_order_id as i64)
                .bind(buyer_id as i64)
                .bind(seller_id as i64)
                .bind(&price)
                .bind(&amount)
                .bind(&base_mint)
                .bind(&quote_mint)
                .bind(timestamp)
                .execute(&mut *tx)
                .await
                .context("Failed to insert trade")?;
            }
            
            DbCommand::UpdateBalance {
                user_id,
                mint,
                available_delta,
                locked_delta,
            } => {
                use crate::shared::database::repositories::cex::UserBalanceRepository;
                use crate::domains::cex::models::balance::UserBalanceUpdate;
                
                let balance_repo = UserBalanceRepository::new(db_pool.clone());
                let update = UserBalanceUpdate {
                    available_delta,
                    locked_delta,
                };
                
                balance_repo.update_balance(user_id, &mint, &update).await
                    .context("Failed to update balance")?;
            }
        }
    }
    
    // 트랜잭션 커밋
    tx.commit().await
        .context("Failed to commit transaction")?;
    
    Ok(())
}


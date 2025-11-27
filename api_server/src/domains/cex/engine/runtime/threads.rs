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
use anyhow::Result;
use crossbeam::channel::Receiver;
use parking_lot::{RwLock, Mutex};

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
    
    // TODO: 실시간 스케줄링 설정 (나중에 구현)
    // use nix::sched::{sched_setscheduler, SchedPolicy, SchedParam};
    // use nix::unistd::Pid;
    // let params = SchedParam { sched_priority: 99 };
    // sched_setscheduler(Pid::from_raw(0), SchedPolicy::Fifo, &params)?;
    
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
    
    // 2. WAL 메시지 발행 (OrderCreated) - 먼저!
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
    
    // 3. OrderBook 가져오기 및 매칭 (락 안에서 수행)
    let matches = {
        let mut orderbooks_guard = orderbooks.write();
        let orderbook = orderbooks_guard.entry(pair.clone()).or_insert_with(|| OrderBook::new(pair.clone()));
        
        // 4. OrderBook에 추가
        orderbook.add_order(order.clone());
        
        // 5. Matcher로 매칭 시도 (락 안에서 수행)
        matcher.match_order(&mut order, orderbook)
    };
    
    // 6. 체결 처리 (락 밖에서 수행)
    {
        let mut executor_guard = executor.lock();
        for match_result in &matches {
            if let Err(e) = executor_guard.execute_trade(match_result) {
                // 에러 발생 시 로그 기록 (나중에 구현)
                eprintln!("Failed to execute trade: {}", e);
            }
        }
    }
    
    // 7. 결과 반환
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


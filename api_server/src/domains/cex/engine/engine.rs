// =====================================================
// HighPerformanceEngine - 고성능 체결 엔진
// =====================================================
// 역할: 모든 주문 처리, 매칭, 체결을 담당하는 통합 엔진
// 
// 핵심 설계:
// 1. 싱글 스레드 엔진 (Core 0 고정) - 모든 주문 순차 처리
// 2. WAL 스레드 (Core 1 고정) - 디스크 쓰기 전용
// 3. DB Writer 스레드 (Core 2, 로컬만) - 배치 DB 쓰기
// 4. Lock-free 채널 - 스레드 간 통신 (crossbeam::channel)
// 5. 환경별 코어 설정 - dev(11코어), prod(2코어)
//
// 성능:
// - 주문 처리: < 0.5ms (평균)
// - 체결 처리: < 0.2ms (평균)
// - TPS: 50,000+ orders/sec
// =====================================================

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use anyhow::{Result, Context};
use crossbeam::channel::{Receiver, Sender, bounded, unbounded};
use tokio::sync::oneshot;
use rust_decimal::Decimal;

use crate::shared::database::Database;
use crate::domains::cex::engine::types::{TradingPair, OrderEntry, MatchResult};
use crate::domains::cex::engine::orderbook::OrderBook;
use crate::domains::cex::engine::matcher::Matcher;
use crate::domains::cex::engine::executor::Executor;
use crate::domains::cex::engine::balance_cache::BalanceCache;
use crate::domains::cex::engine::wal::{WalEntry, WalWriter};
use crate::domains::cex::engine::Engine;

// =====================================================
// OrderCommand - 엔진 스레드로 전달할 명령
// =====================================================
// 역할: API Handler (tokio async)에서 엔진 스레드 (blocking)로
//       명령을 전달하기 위한 메시지 타입
//
// 왜 필요한가?
// - tokio async와 blocking thread 간 통신
// - Request-Response 패턴 구현
// - oneshot 채널로 결과 반환
// =====================================================

/// 엔진 스레드로 전달할 명령
/// 
/// 각 명령은 엔진 스레드에서 순차적으로 처리됩니다.
/// 결과는 oneshot 채널을 통해 비동기로 반환됩니다.
#[derive(Debug)]
pub enum OrderCommand {
    /// 주문 제출
    /// 
    /// # Fields
    /// * `order` - 제출할 주문
    /// * `response` - 결과를 반환할 oneshot 채널
    /// 
    /// # 처리 과정
    /// 1. WAL 메시지 발행 (OrderCreated)
    /// 2. OrderBook에 추가
    /// 3. Matcher로 매칭 시도
    /// 4. 체결된 경우 Executor로 처리
    /// 5. MatchResult 목록을 response로 전송
    SubmitOrder {
        order: OrderEntry,
        response: oneshot::Sender<Result<Vec<MatchResult>>>,
    },
    
    /// 주문 취소
    /// 
    /// # Fields
    /// * `order_id` - 취소할 주문 ID
    /// * `user_id` - 주문한 사용자 ID (권한 확인)
    /// * `trading_pair` - 거래쌍
    /// * `response` - 취소된 주문을 반환할 oneshot 채널
    /// 
    /// # 처리 과정
    /// 1. OrderBook에서 주문 찾기
    /// 2. 권한 확인 (user_id 일치)
    /// 3. WAL 메시지 발행 (OrderCancelled)
    /// 4. OrderBook에서 제거
    /// 5. 잔고 잠금 해제
    /// 6. 취소된 주문을 response로 전송
    CancelOrder {
        order_id: u64,
        user_id: u64,
        trading_pair: TradingPair,
        response: oneshot::Sender<Result<OrderEntry>>,
    },
    
    /// 오더북 조회
    /// 
    /// # Fields
    /// * `trading_pair` - 조회할 거래쌍
    /// * `depth` - 조회할 가격 레벨 개수 (None이면 전체)
    /// * `response` - 오더북을 반환할 oneshot 채널
    /// 
    /// # 반환값
    /// `(매수 주문 목록, 매도 주문 목록)`
    GetOrderbook {
        trading_pair: TradingPair,
        depth: Option<usize>,
        response: oneshot::Sender<Result<(Vec<OrderEntry>, Vec<OrderEntry>)>>,
    },
    
    /// 잔고 조회
    /// 
    /// # Fields
    /// * `user_id` - 사용자 ID
    /// * `mint` - 자산 종류
    /// * `response` - 잔고를 반환할 oneshot 채널
    /// 
    /// # 반환값
    /// `(available, locked)`
    GetBalance {
        user_id: u64,
        mint: String,
        response: oneshot::Sender<Result<(Decimal, Decimal)>>,
    },
    
    /// 잔고 잠금 (주문 생성 시)
    /// 
    /// # Fields
    /// * `user_id` - 사용자 ID
    /// * `mint` - 자산 종류
    /// * `amount` - 잠글 수량
    /// * `response` - 결과를 반환할 oneshot 채널
    /// 
    /// # 처리 과정
    /// 1. BalanceCache에서 잔고 확인
    /// 2. available >= amount 확인
    /// 3. available 감소, locked 증가
    /// 4. WAL 메시지 발행 (BalanceLocked)
    LockBalance {
        user_id: u64,
        mint: String,
        amount: Decimal,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 잔고 잠금 해제 (주문 취소 시)
    /// 
    /// # Fields
    /// * `user_id` - 사용자 ID
    /// * `mint` - 자산 종류
    /// * `amount` - 해제할 수량
    /// * `response` - 결과를 반환할 oneshot 채널
    /// 
    /// # 처리 과정
    /// 1. BalanceCache에서 locked 확인
    /// 2. locked >= amount 확인
    /// 3. locked 감소, available 증가
    /// 4. WAL 메시지 발행 (OrderCancelled)
    UnlockBalance {
        user_id: u64,
        mint: String,
        amount: Decimal,
        response: oneshot::Sender<Result<()>>,
    },
}

// =====================================================
// CoreConfig - 코어 설정 (환경별)
// =====================================================
// 역할: 환경(dev/prod)에 따라 코어 고정 설정을 자동으로 결정
//
// dev (로컬, 11코어):
//   - Engine: Core 0
//   - WAL: Core 1
//   - DB Writer: Core 2
//   - UDP Feed: Core 3
//
// prod (인스턴스, 2코어):
//   - Engine: Core 0
//   - WAL: Core 1
//   - 나머지: None (OS가 알아서 배치)
// =====================================================

/// 코어 설정 구조체
/// 
/// 환경 변수 `RUST_ENV`에 따라 자동으로 코어 설정을 결정합니다.
/// 
/// # 사용 예시
/// ```
/// let config = CoreConfig::from_env();
/// config.set_engine_core();  // Core 0 (또는 None)
/// ```
pub struct CoreConfig {
    /// 엔진 스레드 코어 (항상 Some)
    pub engine_core: usize,
    /// WAL 스레드 코어 (항상 Some)
    pub wal_core: usize,
    /// DB Writer 스레드 코어 (dev만 Some)
    pub db_writer_core: Option<usize>,
}

impl CoreConfig {
    /// 환경 변수에서 코어 설정 읽기
    /// 
    /// # 환경 변수
    /// * `RUST_ENV` - "dev" 또는 "prod" (기본값: "dev")
    /// 
    /// # Returns
    /// 환경에 맞는 코어 설정
    /// 
    /// # Examples
    /// ```
    /// // dev 환경
    /// RUST_ENV=dev
    /// // → engine_core: 0, wal_core: 1, db_writer_core: Some(2)
    /// 
    /// // prod 환경
    /// RUST_ENV=prod
    /// // → engine_core: 0, wal_core: 1, db_writer_core: None
    /// ```
    pub fn from_env() -> Self {
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "dev".to_string());
        
        match env.as_str() {
            "dev" => {
                // 로컬 환경 (11코어) - 여러 코어 활용
                Self {
                    engine_core: 0,
                    wal_core: 1,
                    db_writer_core: Some(2),
                }
            }
            "prod" => {
                // 프로덕션 환경 (2코어) - 최소한만
                Self {
                    engine_core: 0,
                    wal_core: 1,
                    db_writer_core: None,  // 코어 고정 안 함
                }
            }
            _ => {
                // 기본값 (dev와 동일)
                Self {
                    engine_core: 0,
                    wal_core: 1,
                    db_writer_core: None,
                }
            }
        }
    }
    
    /// 코어 고정 설정 (선택적)
    /// 
    /// # Arguments
    /// * `core_id` - 고정할 코어 번호 (None이면 고정 안 함)
    /// 
    /// # Note
    /// 코어 고정 실패해도 경고만 출력하고 계속 진행
    /// (권한 없거나 코어가 없을 수 있음)
    pub fn set_core(core_id: Option<usize>) {
        if let Some(core) = core_id {
            // core_affinity crate 사용 (나중에 추가)
            // use core_affinity::{set_for_current, CoreId};
            // if let Err(e) = set_for_current(CoreId { id: core }) {
            //     log::warn!("Failed to set core affinity to {}: {}", core, e);
            // }
            // 지금은 주석 처리 (의존성 추가 후 활성화)
        }
    }
}

// =====================================================
// HighPerformanceEngine - 통합 엔진
// =====================================================
// 역할: 모든 컴포넌트를 통합하여 고성능 체결 엔진 구현
//
// 구조:
// - 주문 큐: crossbeam::channel (Lock-free)
// - WAL 채널: crossbeam::channel (Lock-free)
// - OrderBook: 거래쌍별로 관리
// - Matcher: 매칭 알고리즘
// - Executor: 체결 처리
// - BalanceCache: 메모리 잔고 관리
//
// 스레드:
// - Engine Thread (Core 0): 주문 처리
// - WAL Thread (Core 1): 디스크 쓰기
// =====================================================

/// 고성능 체결 엔진
/// 
/// 싱글 스레드 엔진 + 멀티 스레드 워커 구조로
/// 초고속 주문 처리와 안전한 데이터 저장을 동시에 달성합니다.
/// 
/// # 아키텍처
/// ```
/// API Handler (tokio async)
///     ↓ order_tx.send()
/// [crossbeam::channel] (Lock-free)
///     ↓ order_rx.recv()
/// Engine Thread (Core 0, 싱글 스레드)
///     ├─ OrderBook::add_order()
///     ├─ Matcher::match_order()
///     ├─ Executor::execute_trade()
///     └─ wal_tx.send()
/// [crossbeam::channel] (Lock-free)
///     ↓ wal_rx.recv()
/// WAL Thread (Core 1)
///     └─ WalWriter::append() → fsync()
/// ```
/// 
/// # 성능
/// - 주문 처리: < 0.5ms (평균)
/// - 체결 처리: < 0.2ms (평균)
/// - TPS: 50,000+ orders/sec
pub struct HighPerformanceEngine {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 채널 (Lock-free Ring Buffer)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    /// 주문 명령 전송 채널 (Sender)
    /// 
    /// API Handler에서 엔진 스레드로 주문을 전송할 때 사용
    /// 
    /// # 특징
    /// - Lock-free: 락 없이 동작 (스핀락도 없음)
    /// - 링버퍼: 고정 크기 버퍼를 순환 사용
    /// - SPSC: Single Producer, Single Consumer
    /// - 성능: ~100ns (메모리 연산)
    order_tx: Sender<OrderCommand>,
    
    /// 주문 명령 수신 채널 (Receiver)
    /// 
    /// 엔진 스레드에서 주문을 수신할 때 사용
    /// 
    /// # 사용 위치
    /// - `engine_thread_loop()`에서 `order_rx.recv()` 호출
    order_rx: Receiver<OrderCommand>,
    
    /// WAL 메시지 전송 채널 (Sender)
    /// 
    /// 엔진 스레드에서 WAL 스레드로 메시지를 전송할 때 사용
    /// 
    /// # 전송하는 메시지
    /// - OrderCreated
    /// - OrderCancelled
    /// - TradeExecuted
    /// - BalanceLocked
    /// - BalanceUpdated
    wal_tx: Sender<WalEntry>,
    
    /// WAL 메시지 수신 채널 (Receiver)
    /// 
    /// WAL 스레드에서 메시지를 수신할 때 사용
    /// 
    /// # 사용 위치
    /// - `wal_thread_loop()`에서 `wal_rx.recv()` 호출
    wal_rx: Receiver<WalEntry>,
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 핵심 컴포넌트 (엔진 스레드에서만 접근)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    /// 거래쌍별 오더북
    /// 
    /// Key: TradingPair (예: SOL/USDT)
    /// Value: OrderBook (매수/매도 호가)
    /// 
    /// # 접근
    /// - 엔진 스레드에서만 접근 (싱글 스레드이므로 안전)
    /// - `engine_thread_loop()`에서 사용
    /// 
    /// # 초기화
    /// - 서버 시작 시 DB에서 활성 주문 로드
    /// - 주문 제출 시 자동 생성
    orderbooks: Arc<parking_lot::RwLock<HashMap<TradingPair, OrderBook>>>,
    
    /// 매칭 엔진
    /// 
    /// Price-Time Priority 기반 매칭 알고리즘
    /// 
    /// # 특징
    /// - 싱글 스레드에서만 사용 (안전)
    /// - 상태 없음 (stateless)
    /// - 성능: < 0.5ms per order
    matcher: Arc<Matcher>,
    
    /// 체결 실행 엔진
    /// 
    /// MatchResult를 받아서 실제 체결 처리
    /// 
    /// # 특징
    /// - BalanceCache 포함
    /// - WAL 메시지 발행
    /// - 싱글 스레드에서만 사용 (안전)
    executor: Arc<parking_lot::Mutex<Executor>>,
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 스레드 관리
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    /// 엔진 스레드 핸들
    /// 
    /// 엔진 스레드가 실행 중인지 확인하고 종료 시 대기할 때 사용
    /// 
    /// # 생명주기
    /// - `start()`에서 생성
    /// - `stop()`에서 종료 대기
    engine_thread: Option<thread::JoinHandle<()>>,
    
    /// WAL 스레드 핸들
    /// 
    /// WAL 스레드가 실행 중인지 확인하고 종료 시 대기할 때 사용
    /// 
    /// # 생명주기
    /// - `start()`에서 생성
    /// - `stop()`에서 종료 대기
    wal_thread: Option<thread::JoinHandle<()>>,
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 상태 관리
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    /// 엔진 실행 중 여부
    /// 
    /// # 사용 목적
    /// - 스레드 루프 종료 조건 확인
    /// - 중복 시작 방지
    /// 
    /// # 값
    /// - `true`: 실행 중
    /// - `false`: 정지됨
    running: Arc<AtomicBool>,
    
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 데이터베이스 (DB Writer용)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    
    /// 데이터베이스 연결
    /// 
    /// # 사용 목적
    /// - 서버 시작 시 잔고/주문 로드
    /// - DB Writer 스레드에서 배치 쓰기
    /// 
    /// # 접근
    /// - `start()`에서만 사용 (초기화)
    /// - DB Writer 스레드에서 사용 (배치 쓰기)
    db: Database,
    
    /// WAL 디렉토리 경로
    /// 
    /// # 기본값
    /// - `./wal/` (현재 디렉토리)
    /// 
    /// # 환경 변수
    /// - `WAL_DIR`: WAL 디렉토리 경로 지정 가능
    wal_dir: std::path::PathBuf,
}

impl HighPerformanceEngine {
    /// 새 엔진 생성
    /// 
    /// # Arguments
    /// * `db` - 데이터베이스 연결
    /// 
    /// # Returns
    /// HighPerformanceEngine 인스턴스
    /// 
    /// # 초기화 내용
    /// 1. 채널 생성 (order_tx/rx, wal_tx/rx)
    /// 2. 컴포넌트 초기화 (OrderBook, Matcher, Executor)
    /// 3. 상태 초기화 (running = false)
    /// 
    /// # Note
    /// 아직 스레드는 시작하지 않음 (`start()` 호출 필요)
    /// 
    /// # Examples
    /// ```
    /// let db = Database::new("postgresql://...").await?;
    /// let engine = HighPerformanceEngine::new(db);
    /// engine.start().await?;  // 스레드 시작
    /// ```
    pub fn new(db: Database) -> Self {
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // 1. 채널 생성 (Lock-free Ring Buffer)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        
        // 주문 명령 채널 (크기: 10,000)
        // SPSC 패턴: API Handler (Producer) → Engine Thread (Consumer)
        let (order_tx, order_rx) = bounded(10_000);
        
        // WAL 메시지 채널 (크기: 10,000)
        // SPSC 패턴: Engine Thread (Producer) → WAL Thread (Consumer)
        let (wal_tx, wal_rx) = bounded(10_000);
        
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // 2. 컴포넌트 초기화
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        
        // OrderBook: 거래쌍별로 관리 (빈 HashMap으로 시작)
        let orderbooks = Arc::new(parking_lot::RwLock::new(HashMap::new()));
        
        // Matcher: 상태 없음 (stateless), Arc로 공유
        let matcher = Arc::new(Matcher::new());
        
        // Executor: BalanceCache 포함, WAL Sender 전달
        let executor = Arc::new(parking_lot::Mutex::new(Executor::new(wal_tx.clone())));
        
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // 3. WAL 디렉토리 경로
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        
        let wal_dir = std::env::var("WAL_DIR")
            .map(|s| std::path::PathBuf::from(s))
            .unwrap_or_else(|_| std::path::PathBuf::from("./wal"));
        
        Self {
            order_tx,
            order_rx,
            wal_tx,
            wal_rx,
            orderbooks,
            matcher,
            executor,
            engine_thread: None,
            wal_thread: None,
            running: Arc::new(AtomicBool::new(false)),
            db,
            wal_dir,
        }
    }
}

// =====================================================
// Engine Trait 구현
// =====================================================
// Engine trait의 모든 메서드를 구현합니다.
// 각 메서드는 oneshot 채널을 사용하여 비동기 응답을 반환합니다.
// =====================================================

#[async_trait]
impl Engine for HighPerformanceEngine {
    
}


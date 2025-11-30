/// ID 생성기
/// ID Generator
///
/// 역할:
/// - 주문 ID 생성 (Order ID)
/// - 체결 ID 생성 (Trade ID)
/// - Atomic counter를 사용하여 스레드 안전하게 ID 생성
///
/// 사용 방법:
/// ```rust
/// let order_id = OrderIdGenerator::next();
/// let trade_id = TradeIdGenerator::next();
/// ```
///
/// 초기화:
/// 서버 시작 시 DB에서 마지막 ID를 읽어와서 초기화
/// (서버 재시작 시에도 ID가 중복되지 않도록)

use std::sync::atomic::{AtomicU64, Ordering};

/// 주문 ID 카운터 (서버 시작 시 초기화)
static ORDER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 체결 ID 카운터 (서버 시작 시 초기화)
static TRADE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 주문 ID 생성기
/// Order ID Generator
///
/// AtomicU64를 사용하여 스레드 안전하게 ID 생성
/// 서버 시작 시 DB에서 마지막 주문 ID를 읽어와서 초기화
pub struct OrderIdGenerator;

impl OrderIdGenerator {
    /// 다음 주문 ID 생성
    /// Generate next order ID
    ///
    /// # Returns
    /// 다음 주문 ID (1부터 시작, 자동 증가)
    ///
    /// # Thread Safety
    /// AtomicU64를 사용하여 스레드 안전하게 동작
    pub fn next() -> u64 {
        ORDER_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// ID 생성기 초기화
    /// Initialize ID generator
    ///
    /// 서버 시작 시 DB에서 마지막 주문 ID를 읽어와서 초기화
    /// 이렇게 하면 서버 재시작 시에도 ID가 중복되지 않음
    ///
    /// # Arguments
    /// * `last_id` - DB에서 읽은 마지막 주문 ID (없으면 0)
    ///
    /// # Example
    /// ```rust
    /// let last_order_id = db.get_max_order_id().await?;
    /// OrderIdGenerator::initialize(last_order_id);
    /// ```
    pub fn initialize(last_id: u64) {
        // 마지막 ID 다음부터 시작
        ORDER_ID_COUNTER.store(last_id + 1, Ordering::SeqCst);
    }

    /// 현재 ID 값 조회 (디버깅용)
    /// Get current ID value (for debugging)
    pub fn current() -> u64 {
        ORDER_ID_COUNTER.load(Ordering::SeqCst)
    }
}

/// 체결 ID 생성기
/// Trade ID Generator
///
/// AtomicU64를 사용하여 스레드 안전하게 ID 생성
/// 서버 시작 시 DB에서 마지막 체결 ID를 읽어와서 초기화
pub struct TradeIdGenerator;

impl TradeIdGenerator {
    /// 다음 체결 ID 생성
    /// Generate next trade ID
    ///
    /// # Returns
    /// 다음 체결 ID (1부터 시작, 자동 증가)
    ///
    /// # Thread Safety
    /// AtomicU64를 사용하여 스레드 안전하게 동작
    pub fn next() -> u64 {
        TRADE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// ID 생성기 초기화
    /// Initialize ID generator
    ///
    /// 서버 시작 시 DB에서 마지막 체결 ID를 읽어와서 초기화
    ///
    /// # Arguments
    /// * `last_id` - DB에서 읽은 마지막 체결 ID (없으면 0)
    pub fn initialize(last_id: u64) {
        // 마지막 ID 다음부터 시작
        TRADE_ID_COUNTER.store(last_id + 1, Ordering::SeqCst);
    }

    /// 현재 ID 값 조회 (디버깅용)
    /// Get current ID value (for debugging)
    pub fn current() -> u64 {
        TRADE_ID_COUNTER.load(Ordering::SeqCst)
    }
}


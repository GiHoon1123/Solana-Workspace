// =====================================================
// Runtime - 통합 엔진 런타임
// =====================================================
// 역할: 고성능 체결 엔진의 실행 시점 관리
//
// 구조:
// - main.rs: HighPerformanceEngine 구조체
// - commands.rs: OrderCommand enum
// - config.rs: CoreConfig (환경별 코어 설정)
// - threads.rs: 스레드 루프 함수들 (나중에 구현)
// =====================================================

pub mod config;
pub mod commands;
pub mod engine;
pub mod threads;
pub mod db_commands;

pub use engine::HighPerformanceEngine;
pub use config::CoreConfig;
pub use commands::OrderCommand;
pub use db_commands::DbCommand;
pub use threads::{engine_thread_loop, wal_thread_loop, db_writer_thread_loop};


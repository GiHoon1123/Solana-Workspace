// Errors module: 커스텀 에러 타입
// 역할: 컴파일 타임 에러 타입 안정성 보장
// Errors module: custom error types for compile-time safety

pub mod wallet_error;
pub mod auth_error;

pub use wallet_error::*;
pub use auth_error::*;


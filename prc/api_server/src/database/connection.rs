use sqlx::PgPool;
use anyhow::{Context, Result};

// 데이터베이스 연결 풀
// 역할: NestJS의 Database connection 같은 것
// Database connection pool for PostgreSQL
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    // 데이터베이스 연결 생성
    // Create database connection
    // db_url: PostgreSQL 연결 문자열 (예: "postgresql://root:1234@localhost/solana_api")
    pub async fn new(db_url: &str) -> Result<Self> {
        // PostgreSQL 연결 풀 생성
        // Create PostgreSQL connection pool
        let pool = PgPool::connect(db_url)
            .await
            .context("Failed to connect to database")?;

        Ok(Self { pool })
    }

    // 연결 풀 반환
    // Get connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // 테이블 생성 (초기화)
    // Create tables (initialization)
    pub async fn initialize(&self) -> Result<()> {
        // transactions 테이블 생성
        // Create transactions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id BIGSERIAL PRIMARY KEY,
                input_mint VARCHAR(255) NOT NULL,
                output_mint VARCHAR(255) NOT NULL,
                amount BIGINT NOT NULL,
                expected_out_amount BIGINT,
                user_public_key VARCHAR(255) NOT NULL,
                transaction_bytes TEXT NOT NULL,
                quote_response JSONB,
                status VARCHAR(50) NOT NULL DEFAULT 'created',
                created_at TIMESTAMP NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(self.pool())
        .await
        .context("Failed to create transactions table")?;
    
        println!("Database initialized successfully");
        Ok(())
    }
}
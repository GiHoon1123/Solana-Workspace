use sqlx::{PgPool, Row};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use crate::domains::cex::models::trade::{Trade, TradeCreate};

pub struct TradeRepository {
    pool: PgPool,
}

impl TradeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 체결 내역 생성
    /// Create trade
    pub async fn create(&self, trade_create: &TradeCreate) -> Result<Trade> {
        let row = sqlx::query(
            r#"
            INSERT INTO trades (
                buy_order_id, sell_order_id, base_mint, quote_mint,
                price, amount, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, buy_order_id, sell_order_id, base_mint, quote_mint,
                      price, amount, created_at
            "#,
        )
        .bind(trade_create.buy_order_id as i64)
        .bind(trade_create.sell_order_id as i64)
        .bind(&trade_create.base_mint)
        .bind(&trade_create.quote_mint)
        .bind(&trade_create.price)
        .bind(&trade_create.amount)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .context("Failed to create trade")?;

        Ok(self.row_to_trade(&row))
    }

    /// 체결 내역 ID로 조회
    /// Get trade by ID
    pub async fn get_by_id(&self, trade_id: u64) -> Result<Option<Trade>> {
        let row = sqlx::query(
            r#"
            SELECT id, buy_order_id, sell_order_id, base_mint, quote_mint,
                   price, amount, created_at
            FROM trades
            WHERE id = $1
            "#,
        )
        .bind(trade_id as i64)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch trade by id")?;

        Ok(row.map(|r| self.row_to_trade(&r)))
    }

    /// 거래쌍별 체결 내역 조회 (최신순)
    /// Get trades by trading pair (sorted by time, newest first)
    pub async fn get_by_pair(
        &self,
        base_mint: &str,
        quote_mint: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Trade>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query(
            r#"
            SELECT id, buy_order_id, sell_order_id, base_mint, quote_mint,
                   price, amount, created_at
            FROM trades
            WHERE base_mint = $1 AND quote_mint = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(base_mint)
        .bind(quote_mint)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch trades by pair")?;

        Ok(rows.iter().map(|r| self.row_to_trade(r)).collect())
    }

    /// 매수 주문 ID로 체결 내역 조회
    /// Get trades by buy order ID
    pub async fn get_by_buy_order(
        &self,
        buy_order_id: u64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Trade>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query(
            r#"
            SELECT id, buy_order_id, sell_order_id, base_mint, quote_mint,
                   price, amount, created_at
            FROM trades
            WHERE buy_order_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(buy_order_id as i64)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch trades by buy order")?;

        Ok(rows.iter().map(|r| self.row_to_trade(r)).collect())
    }

    /// 매도 주문 ID로 체결 내역 조회
    /// Get trades by sell order ID
    pub async fn get_by_sell_order(
        &self,
        sell_order_id: u64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Trade>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query(
            r#"
            SELECT id, buy_order_id, sell_order_id, base_mint, quote_mint,
                   price, amount, created_at
            FROM trades
            WHERE sell_order_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(sell_order_id as i64)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch trades by sell order")?;

        Ok(rows.iter().map(|r| self.row_to_trade(r)).collect())
    }

    /// 최신 체결 내역 조회 (모든 거래쌍)
    /// Get recent trades (all trading pairs)
    pub async fn get_recent_trades(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<Trade>> {
        let limit = limit.unwrap_or(100);

        let rows = sqlx::query(
            r#"
            SELECT id, buy_order_id, sell_order_id, base_mint, quote_mint,
                   price, amount, created_at
            FROM trades
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch recent trades")?;

        Ok(rows.iter().map(|r| self.row_to_trade(r)).collect())
    }

    /// Row를 Trade로 변환하는 헬퍼 메서드
    /// Helper method to convert Row to Trade
    fn row_to_trade(&self, row: &sqlx::postgres::PgRow) -> Trade {
        Trade {
            id: row.get::<i64, _>("id") as u64,
            buy_order_id: row.get::<i64, _>("buy_order_id") as u64,
            sell_order_id: row.get::<i64, _>("sell_order_id") as u64,
            base_mint: row.get("base_mint"),
            quote_mint: row.get("quote_mint"),
            price: row.get("price"),
            amount: row.get("amount"),
            created_at: row.get("created_at"),
        }
    }
}


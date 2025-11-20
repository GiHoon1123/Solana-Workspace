-- Transactions 테이블
-- 스왑 트랜잭션 정보 저장
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
);


-- Solana Wallets 테이블
-- 사용자 Solana 지갑 정보 (서버에서 키 관리)
-- Note: FK 제약조건 없음, 논리적 관계만 유지
CREATE TABLE IF NOT EXISTS solana_wallets (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    public_key VARCHAR(255) UNIQUE NOT NULL,
    encrypted_private_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 인덱스 추가
CREATE INDEX IF NOT EXISTS idx_solana_wallets_user_id ON solana_wallets(user_id);
CREATE INDEX IF NOT EXISTS idx_solana_wallets_public_key ON solana_wallets(public_key);


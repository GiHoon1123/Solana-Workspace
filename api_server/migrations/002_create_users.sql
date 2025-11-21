-- Users 테이블
-- 사용자 계정 정보 (로그인/회원가입)
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    username VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 인덱스 추가 (이메일로 빠른 조회)
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);


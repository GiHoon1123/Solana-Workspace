use crate::database::{Database, UserRepository};
use crate::models::{User, SignupRequest, SigninRequest};
use crate::errors::AuthError;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

// 인증 서비스
// 역할: NestJS의 Service 같은 것
// AuthService: handles authentication business logic
#[derive(Clone)]
pub struct AuthService {
    db: Database,
}

impl AuthService {
    // 생성자
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // 회원가입 (비즈니스 로직)
    pub async fn signup(
        &self,
        request: SignupRequest,
    ) -> Result<User, AuthError> {
        // Repository 생성 (Service 내부에서)
        let user_repo = UserRepository::new(self.db.pool().clone());

        // 1. 이메일 중복 확인
        let existing_user = user_repo
            .get_user_by_email(&request.email)
            .await
            .map_err(|e| AuthError::DatabaseError(format!("Failed to check email existence: {}", e)))?;
        
        if existing_user.is_some() {
            return Err(AuthError::EmailAlreadyExists { email: request.email });
        }

        // 2. 비밀번호 해싱
        let password_hash = Self::hash_password(&request.password)?;

        // 3. 사용자 생성
        let user = user_repo
            .create_user(
                &request.email,
                &password_hash,
                request.username.as_deref(),
            )
            .await
            .map_err(|e| AuthError::DatabaseError(format!("Failed to create user: {}", e)))?;

        Ok(user)
    }

    // 로그인 (비즈니스 로직)
    pub async fn signin(
        &self,
        request: SigninRequest,
    ) -> Result<User, AuthError> {
        // Repository 생성 (Service 내부에서)
        let user_repo = UserRepository::new(self.db.pool().clone());

        // 1. 이메일로 사용자 조회
        let user = user_repo
            .get_user_by_email(&request.email)
            .await
            .map_err(|e| AuthError::DatabaseError(format!("Failed to fetch user: {}", e)))?;

        let user = match user {
            Some(u) => u,
            None => return Err(AuthError::InvalidCredentials),
        };

        // 2. 비밀번호 검증
        Self::verify_password(&request.password, &user.password_hash)?;

        Ok(user)
    }

    fn hash_password(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHashingFailed(format!("Failed to hash password: {}", e)))?
            .to_string();

        Ok(password_hash)
    }

    fn verify_password(password: &str, password_hash: &str) -> Result<(), AuthError> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|e| AuthError::PasswordVerificationFailed(format!("Invalid password hash: {}", e)))?;

        let argon2 = Argon2::default();
        
        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(())
    }
}
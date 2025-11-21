use crate::clients::SolanaClient;
use crate::database::{Database, SolanaWalletRepository};
use crate::models::SolanaWallet;
use anyhow::{Context, Result};
use solana_sdk::{
    signer::keypair::Keypair,
    signer::Signer,
    pubkey::Pubkey,
    transaction::Transaction,
};
use base64::{Engine as _, engine::general_purpose};

/// 지갑 서비스
/// 역할: NestJS의 Service 같은 것
/// WalletService: handles wallet-related business logic
#[derive(Clone)]
pub struct WalletService {
    db: Database,
    solana_client: SolanaClient,
}

impl WalletService {
    /// 생성자
    /// Constructor
    pub fn new(db: Database) -> Result<Self> {
        let solana_client = SolanaClient::new()
            .context("Failed to create SolanaClient")?;

        Ok(Self {
            db,
            solana_client,
        })
    }

    /// 새 지갑 생성
    /// Create new wallet for user
    pub async fn create_wallet(&self, user_id: u64) -> Result<SolanaWallet> {
        // 1. Keypair 생성 (새 지갑)
        let keypair = SolanaClient::generate_wallet();
        let public_key = keypair.pubkey().to_string();

        // 2. Private Key 암호화 (일단 Base64 인코딩, 나중에 실제 암호화 추가)
        // Encrypt private key (currently Base64 encoding, will add real encryption later)
        let private_key_bytes = keypair.to_bytes();
        let encrypted_private_key = general_purpose::STANDARD.encode(&private_key_bytes);

        // 3. DB에 저장
        let wallet_repo = SolanaWalletRepository::new(self.db.pool().clone());
        let wallet = wallet_repo
            .create_solana_wallet(user_id, &public_key, &encrypted_private_key)
            .await
            .context("Failed to save wallet to database")?;

        Ok(wallet)
    }

    /// 지갑 조회 (ID로)
    /// Get wallet by ID
    pub async fn get_wallet(&self, wallet_id: u64) -> Result<SolanaWallet> {
        let wallet_repo = SolanaWalletRepository::new(self.db.pool().clone());
        let wallet = wallet_repo
            .get_solana_wallet_by_id(wallet_id)
            .await
            .context("Failed to fetch wallet")?;

        wallet.ok_or_else(|| anyhow::anyhow!("Wallet not found: id={}", wallet_id))
    }

    /// 사용자의 모든 지갑 조회
    /// Get all wallets for user
    pub async fn get_user_wallets(&self, user_id: u64) -> Result<Vec<SolanaWallet>> {
        let wallet_repo = SolanaWalletRepository::new(self.db.pool().clone());
        let wallets = wallet_repo
            .get_solana_wallets_by_user_id(user_id)
            .await
            .context("Failed to fetch user wallets")?;

        Ok(wallets)
    }

    /// Public Key로 지갑 조회
    /// Get wallet by public key
    pub async fn get_wallet_by_public_key(&self, public_key: &str) -> Result<SolanaWallet> {
        let wallet_repo = SolanaWalletRepository::new(self.db.pool().clone());
        let wallet = wallet_repo
            .get_solana_wallet_by_public_key(public_key)
            .await
            .context("Failed to fetch wallet")?;

        wallet.ok_or_else(|| anyhow::anyhow!("Wallet not found: public_key={}", public_key))
    }

    /// 지갑 잔액 조회 (lamports)
    /// Get wallet balance (in lamports)
    pub async fn get_balance(&self, wallet_id: u64) -> Result<u64> {
        // 1. 지갑 조회
        let wallet = self.get_wallet(wallet_id).await?;
        let pubkey = SolanaClient::parse_pubkey(&wallet.public_key)
            .context("Failed to parse wallet public key")?;

        // 2. Solana 네트워크에서 잔액 조회
        let balance = self
            .solana_client
            .get_balance(&pubkey)
            .await
            .context("Failed to get balance from Solana network")?;

        Ok(balance)
    }

    /// 지갑 잔액 조회 (SOL)
    /// Get wallet balance (in SOL)
    pub async fn get_balance_sol(&self, wallet_id: u64) -> Result<f64> {
        let balance_lamports = self.get_balance(wallet_id).await?;
        let balance_sol = balance_lamports as f64 / 1_000_000_000.0; // lamports to SOL
        Ok(balance_sol)
    }

    /// Private Key 복호화 (Keypair로 변환)
    /// Decrypt private key (convert to Keypair)
    /// Note: 현재는 Base64 디코딩, 나중에 실제 복호화 추가
    pub fn decrypt_private_key(encrypted_private_key: &str) -> Result<Keypair> {
        // Base64 디코딩
        let private_key_bytes = general_purpose::STANDARD
            .decode(encrypted_private_key)
            .context("Failed to decode private key")?;

        // Keypair로 변환
        let keypair = Keypair::from_bytes(&private_key_bytes)
            .context("Failed to create keypair from private key bytes")?;

        Ok(keypair)
    }

    /// SOL 전송
    /// Transfer SOL from one wallet to another
    pub async fn transfer_sol(
        &self,
        from_wallet_id: u64,
        to_public_key: &str,
        amount_lamports: u64,
    ) -> Result<String> {
        // 1. 송신 지갑 조회
        let from_wallet = self.get_wallet(from_wallet_id).await?;

        // 2. Private Key 복호화하여 Keypair 생성
        let from_keypair = Self::decrypt_private_key(&from_wallet.encrypted_private_key)
            .context("Failed to decrypt sender private key")?;

        // 3. 수신 Public Key 파싱
        let to_pubkey = SolanaClient::parse_pubkey(to_public_key)
            .context("Failed to parse recipient public key")?;

        // 4. 트랜잭션 생성
        let transaction = self
            .solana_client
            .create_transfer_transaction(&from_keypair, &to_pubkey, amount_lamports)
            .await
            .context("Failed to create transfer transaction")?;

        // 5. 트랜잭션 전송
        let signature = self
            .solana_client
            .send_and_confirm_transaction(&transaction)
            .await
            .context("Failed to send transaction")?;

        Ok(signature)
    }

    /// 트랜잭션 상태 확인
    /// Get transaction status
    pub async fn get_transaction_status(&self, signature: &str) -> Result<Option<bool>> {
        let status = self
            .solana_client
            .get_signature_status(signature)
            .await
            .context("Failed to get transaction status")?;

        Ok(status)
    }
}


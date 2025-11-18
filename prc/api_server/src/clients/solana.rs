use anyhow::{Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    account::Account,
};
use anchor_client::{
    Client as AachorClient,
    Cluster,
    Program,

};
use std::str::FromStr;
use std::sync::Arc;


pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
    rpc_url: String,
    commitment: CommitmentConfig,

}

impl SolanaClient {
    pub fn new(rpc_url: Option<String>) -> Result<Self> {
        let url = rpc_url.unwrap_or_else(|| "https://api.devnet.solana.com".to_string());

        let rpc_client = Arc::new(
            RpcClient::new_with_commitment(
                url.clone(),
                CommitmentConfig::confirmed(),
            )
        );

        Ok(Self {
            rpc_client,
            rpc_url: url,
            commitment: CommitmentConfig::confirmed(),
        })
    }
}
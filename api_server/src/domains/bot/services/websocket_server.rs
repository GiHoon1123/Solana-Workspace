use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use anyhow::{Context, Result};

/// 바이낸스 형식의 오더북 메시지
/// Binance-format orderbook message
/// 
/// 프론트엔드가 기대하는 형식과 동일하게 맞춤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookMessage {
    /// 이벤트 타입 (항상 "depthUpdate")
    #[serde(rename = "e")]
    pub event_type: String,
    
    /// 이벤트 시간 (밀리초)
    #[serde(rename = "E")]
    pub event_time: u64,
    
    /// 심볼 (예: "SOLUSDT")
    #[serde(rename = "s")]
    pub symbol: String,
    
    /// 첫 번째 업데이트 ID
    #[serde(rename = "U")]
    pub first_update_id: u64,
    
    /// 마지막 업데이트 ID
    #[serde(rename = "u")]
    pub last_update_id: u64,
    
    /// 매수 호가 (가격, 수량 쌍)
    /// Bids: [[price, quantity], ...]
    /// 주의: Binance는 Vec<Vec<String>> 형식을 사용합니다
    #[serde(rename = "b")]
    pub bids: Vec<Vec<String>>,
    
    /// 매도 호가 (가격, 수량 쌍)
    /// Asks: [[price, quantity], ...]
    /// 주의: Binance는 Vec<Vec<String>> 형식을 사용합니다
    #[serde(rename = "a")]
    pub asks: Vec<Vec<String>>,
}

/// WebSocket 서버
/// WebSocket Server
/// 
/// 역할:
/// - 프론트엔드 WebSocket 연결 수락
/// - 오더북 업데이트 브로드캐스트
/// - 바이낸스와 동일한 형식으로 메시지 전송
/// 
/// 처리 흐름:
/// 1. 클라이언트 WebSocket 연결 수락
/// 2. 오더북 업데이트 수신 (broadcast 채널)
/// 3. 바이낸스 형식으로 변환
/// 4. 클라이언트로 전송
pub struct WebSocketServer {
    /// 오더북 업데이트 브로드캐스트 채널 (송신자)
    /// Orderbook update broadcast channel (sender)
    pub update_tx: broadcast::Sender<OrderbookMessage>,
    
    /// 업데이트 ID 카운터
    /// Update ID counter
    update_id: Arc<RwLock<u64>>,
}

impl WebSocketServer {
    /// 생성자
    /// Constructor
    /// 
    /// # Arguments
    /// * `channel_capacity` - 브로드캐스트 채널 용량 (기본: 100)
    /// 
    /// # Returns
    /// WebSocketServer 인스턴스
    pub fn new(channel_capacity: usize) -> Self {
        let (update_tx, _) = broadcast::channel(channel_capacity);
        
        Self {
            update_tx,
            update_id: Arc::new(RwLock::new(0)),
        }
    }

    /// WebSocket 연결 처리
    /// Handle WebSocket connection
    /// 
    /// 클라이언트와의 WebSocket 연결을 처리하고, 오더북 업데이트를 전송합니다.
    /// 
    /// # Arguments
    /// * `stream` - WebSocket 스트림 (tokio_tungstenite)
    /// * `update_tx` - 오더북 업데이트 브로드캐스트 채널
    /// * `update_id` - 업데이트 ID 카운터
    pub async fn handle_websocket_connection(
        stream: tokio::net::TcpStream,
        update_tx: broadcast::Sender<OrderbookMessage>,
        _update_id: Arc<RwLock<u64>>,
    ) -> Result<()> {
        // WebSocket 업그레이드
        let ws_stream = accept_async(stream)
            .await
            .context("Failed to accept WebSocket connection")?;
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let mut rx = update_tx.subscribe();
        
        // 클라이언트로 메시지 전송 태스크
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("[WebSocket Server] Failed to serialize message: {}", e);
                        continue;
                    }
                };
                
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    // 클라이언트 연결 끊어짐
                    break;
                }
            }
        });
        
        // 클라이언트로부터 메시지 수신 태스크 (필요시)
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                if let Message::Close(_) = msg {
                    // 클라이언트가 연결 종료
                    break;
                }
                // 다른 메시지는 무시 (필요시 처리)
            }
        });
        
        // 둘 중 하나가 종료되면 전체 종료
        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        };
        
        Ok(())
    }

    /// 오더북 업데이트 브로드캐스트
    /// Broadcast orderbook update
    /// 
    /// 오더북 동기화 서비스에서 호출하여 모든 연결된 클라이언트에 오더북을 전송합니다.
    /// 
    /// # Arguments
    /// * `bids` - 매수 호가 리스트 (가격, 수량)
    /// * `asks` - 매도 호가 리스트 (가격, 수량)
    /// * `symbol` - 심볼 (예: "SOLUSDT")
    /// 
    /// # Returns
    /// * `Ok(())` - 브로드캐스트 성공
    /// * `Err` - 브로드캐스트 실패
    pub async fn broadcast_orderbook(
        &self,
        bids: Vec<(Decimal, Decimal)>,
        asks: Vec<(Decimal, Decimal)>,
        symbol: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 업데이트 ID 증가
        let mut id = self.update_id.write().await;
        *id += 1;
        let current_id = *id;
        drop(id);
        
        // 바이낸스 형식으로 변환 (Vec<Vec<String>>)
        let bids_str: Vec<Vec<String>> = bids
            .iter()
            .map(|(price, qty)| vec![
                price.to_string(),
                qty.to_string(),
            ])
            .collect();
        
        let asks_str: Vec<Vec<String>> = asks
            .iter()
            .map(|(price, qty)| vec![
                price.to_string(),
                qty.to_string(),
            ])
            .collect();
        
        let message = OrderbookMessage {
            event_type: "depthUpdate".to_string(),
            event_time: chrono::Utc::now().timestamp_millis() as u64,
            symbol: symbol.to_string(),
            first_update_id: current_id,
            last_update_id: current_id,
            bids: bids_str,
            asks: asks_str,
        };
        
        // 브로드캐스트 (에러는 무시 - 구독자가 없을 수 있음)
        let _ = self.update_tx.send(message);
        
        Ok(())
    }
}


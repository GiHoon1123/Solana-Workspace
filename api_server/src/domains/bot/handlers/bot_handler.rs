use axum::{extract::State, Json, response::IntoResponse, body::Body, http::Request};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use futures_util::{SinkExt, StreamExt};
use utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use crate::shared::services::AppState;
use crate::shared::database::UserRepository;

/// 봇 데이터 삭제 요청
/// Delete bot data request
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteBotDataRequest {
    /// 삭제할 봇 이메일 (bot1@bot.com 또는 bot2@bot.com)
    /// Bot email to delete data for
    pub bot_email: String,
}

/// 봇 데이터 삭제 응답
/// Delete bot data response
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteBotDataResponse {
    /// 삭제된 주문 수
    /// Number of deleted orders
    pub deleted_orders: u64,
    
    /// 삭제된 거래 수
    /// Number of deleted trades
    pub deleted_trades: u64,
    
    /// 메시지
    /// Message
    pub message: String,
}

/// 봇 데이터 삭제
/// Delete bot data
/// 
/// 특정 봇의 모든 주문과 거래 내역을 삭제합니다.
/// 
/// # Arguments
/// * `State(app_state)` - 애플리케이션 상태
/// * `Json(request)` - 삭제 요청 (봇 이메일)
/// 
/// # Returns
/// * `Json(DeleteBotDataResponse)` - 삭제 결과
/// 
/// # Errors
/// - 봇 이메일이 유효하지 않음
/// - 데이터베이스 오류
#[utoipa::path(
    delete,
    path = "/api/bot/data",
    request_body = DeleteBotDataRequest,
    responses(
        (status = 200, description = "봇 데이터 삭제 성공", body = DeleteBotDataResponse),
        (status = 400, description = "잘못된 요청"),
        (status = 500, description = "서버 오류")
    ),
    tag = "Bot"
)]
pub async fn delete_bot_data(
    State(app_state): State<AppState>,
    Json(request): Json<DeleteBotDataRequest>,
) -> axum::response::Response {
    // 데이터베이스 연결 가져오기
    let db = app_state.db.clone();
    
    // 봇 이메일로 사용자 ID 조회
    let user_repo = UserRepository::new(db.pool().clone());
    let user = match user_repo.get_user_by_email(&request.bot_email).await {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[Bot Handler] Failed to find user: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };
    
    let user_id = match user {
        Some(u) => u.id,
        None => {
            return (axum::http::StatusCode::NOT_FOUND, 
                axum::Json(serde_json::json!({
                    "error": format!("Bot not found: {}", request.bot_email)
                }))).into_response();
        }
    };
    
    // 1. 봇의 주문 ID 목록 먼저 조회 (거래 삭제를 위해 필요)
    let bot_order_ids: Vec<i64> = match sqlx::query(
        "SELECT id FROM orders WHERE user_id = $1"
    )
    .bind(user_id as i64)
    .fetch_all(db.pool())
    .await
    {
        Ok(rows) => rows.iter().map(|row| row.get::<i64, _>("id")).collect(),
        Err(e) => {
            eprintln!("[Bot Handler] Failed to fetch bot orders: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };
    
    // 2. 봇이 참여한 거래 삭제 (주문 삭제 전에 먼저 삭제)
    let deleted_trades = if !bot_order_ids.is_empty() {
        match sqlx::query(
            "DELETE FROM trades WHERE buy_order_id = ANY($1) OR sell_order_id = ANY($1) RETURNING id"
        )
        .bind(&bot_order_ids)
        .fetch_all(db.pool())
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("[Bot Handler] Failed to delete trades: {}", e);
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        }
    } else {
        vec![]
    };
    
    // 3. 봇의 주문 삭제
    let deleted_orders = match sqlx::query(
        "DELETE FROM orders WHERE user_id = $1 RETURNING id"
    )
    .bind(user_id as i64)
    .fetch_all(db.pool())
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("[Bot Handler] Failed to delete orders: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };
    
    (axum::http::StatusCode::OK, axum::Json(DeleteBotDataResponse {
        deleted_orders: deleted_orders.len() as u64,
        deleted_trades: deleted_trades.len() as u64,
        message: format!(
            "Deleted {} orders and {} trades for bot {}",
            deleted_orders.len(),
            deleted_trades.len(),
            request.bot_email
        ),
    })).into_response()
}

/// WebSocket 핸들러
/// WebSocket handler for orderbook updates
/// 
/// 프론트엔드에서 오더북 WebSocket 연결을 처리합니다.
/// 
/// # WebSocket 주소
/// `ws://localhost:3002/api/bot/ws/orderbook`
/// 
/// # 메시지 형식
/// 바이낸스 depth stream과 동일한 형식:
/// ```json
/// {
///   "e": "depthUpdate",
///   "E": 1234567890,
///   "s": "SOLUSDT",
///   "U": 1,
///   "u": 1,
///   "b": [["136.60", "1.0"], ...],
///   "a": [["136.70", "1.0"], ...]
/// }
/// ```
pub async fn handle_websocket(
    State(app_state): State<AppState>,
    req: Request<Body>,
) -> axum::response::Response {
    use axum::response::Response;
    use axum::body::Body as AxumBody;
    use hyper::upgrade::OnUpgrade;
    use tokio_tungstenite::WebSocketStream;
    use tokio_tungstenite::tungstenite::protocol::Role;
    
    let ws_server = app_state.bot_ws_server.clone();
    
    // HTTP 업그레이드 헤더 확인
    if !req.headers().contains_key("upgrade") {
        return Response::builder()
            .status(400)
            .body(AxumBody::from("Not a WebSocket request"))
            .unwrap();
    }
    
    // WebSocket 업그레이드 처리
    let (mut parts, _) = req.into_parts();
    
    // upgrade 확장 가져오기
    let upgrade = match parts.extensions.remove::<OnUpgrade>() {
        Some(upgrade) => upgrade,
        None => {
            return Response::builder()
                .status(500)
                .body(AxumBody::from("Upgrade not available"))
                .unwrap();
        }
    };
    
    // WebSocket 연결 처리
    tokio::spawn(async move {
        let upgraded = match upgrade.await {
            Ok(upgraded) => upgraded,
            Err(_) => {
                eprintln!("[WebSocket Handler] Upgrade failed");
                return;
            }
        };
        
        // Upgraded를 WebSocket 스트림으로 변환
        // hyper::upgrade::Upgraded를 tokio::io::AsyncRead + AsyncWrite로 변환
        use hyper_util::rt::TokioIo;
        
        // Upgraded를 TokioIo로 래핑하여 tokio::io::AsyncRead + AsyncWrite로 변환
        let io = TokioIo::new(upgraded);
        
        // WebSocket 스트림 생성
        let ws_stream = WebSocketStream::from_raw_socket(
            io,
            Role::Server,
            None,
        ).await;
        
        let (mut sender, mut receiver) = ws_stream.split();
        let mut rx = ws_server.update_tx.subscribe();
        
        // 클라이언트 연결 (로그 제거)
        
        // 오더북 업데이트를 클라이언트로 전송하는 태스크
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("[WebSocket Handler] Failed to serialize message: {}", e);
                        continue;
                    }
                };
                
                if sender.send(WsMessage::Text(json)).await.is_err() {
                    // 클라이언트 연결 끊어짐
                    // 클라이언트 연결 끊어짐
                    break;
                }
            }
        });
        
        // 클라이언트로부터 메시지 수신 태스크 (필요시)
        // sender는 send_task에서 사용되므로, recv_task에서는 사용하지 않습니다.
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    WsMessage::Close(_) => {
                        // 클라이언트가 연결 종료
                        // 클라이언트 연결 종료
                        break;
                    }
                    WsMessage::Ping(_data) => {
                        // Ping에 대한 Pong 응답은 send_task에서 처리할 수 없으므로
                        // 여기서는 무시합니다 (필요시 별도 처리)
                    }
                    _ => {
                        // 다른 메시지는 무시 (필요시 처리)
                    }
                }
            }
        });
        
        // 둘 중 하나가 종료되면 전체 종료
        tokio::select! {
            _ = (&mut send_task) => {
                recv_task.abort();
            }
            _ = (&mut recv_task) => {
                send_task.abort();
            }
        };
        
        // WebSocket 연결 종료
    });
    
    // WebSocket 업그레이드 응답
    // Sec-WebSocket-Accept 헤더는 hyper가 자동으로 처리합니다.
    Response::builder()
        .status(101)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .body(AxumBody::empty())
        .unwrap()
}


use axum::Router;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Routes import
mod routes;
mod handlers;
mod models;
mod clients;  // 외부 API 클라이언트 모듈 추가

use routes::create_router;
use crate::models::{QuoteRequest, QuoteResponse, RoutePlan, SwapInfo};
use crate::handlers::swap_handler;

// OpenAPI 스키마 정의: Swagger 문서 자동 생성
// OpenAPI schema definition: auto-generate Swagger documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        swap_handler::get_quote
    ),
    components(schemas(
        QuoteRequest,
        QuoteResponse,
        RoutePlan,
        SwapInfo
    )),
    tags(
        (name = "Swap", description = "Swap API endpoints (Jupiter integration)")
    ),
    info(
        title = "Solana API Server",
        description = "API server for Solana blockchain interactions",
        version = "1.0.0"
    )
)]
struct ApiDoc;

// Main entry point: 프로그램 시작점
// 역할: NestJS의 main.ts와 동일
#[tokio::main]
async fn main() {
    // Router 생성: 라우팅 설정
    // 역할: NestJS의 app.use('/api', router) 같은 것
    // Swagger UI 추가: /swagger-ui 경로에서 API 문서 확인 가능
    // Add Swagger UI: API documentation available at /swagger-ui
    let app = Router::new()
        .merge(create_router())
        .merge(
            SwaggerUi::new("/api")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
        );

    // 서버 시작: 3002 포트에서 리스닝
    let listener = TcpListener::bind("0.0.0.0:3002")
        .await
        .unwrap();
    
    println!("Server running on http://localhost:3002");
    println!("Swagger UI available at http://localhost:3002/api");
    
    // 서버 실행
    axum::serve(listener, app)
        .await
        .unwrap();
}
use axum::{Router, routing::get};
use tokio::net::TcpListener;

// Handler function: 요청을 처리하는 함수
// NestJS의 @Get() 핸들러 같은 역할
// Handler function: handles incoming requests
// 역할: NestJS의 @Get() 핸들러와 동일
async fn handler() -> &'static str {
    "Hello, Axum!"  // 응답: 단순 문자열 반환
    // Response: simple string return
}

// Main entry point: 프로그램 시작점
// 역할: NestJS의 main.ts와 동일
// Main entry point: program starting point
#[tokio::main]
async fn main() {
    // Router 생성: 라우팅 설정
    // 역할: NestJS의 app.get("/", ...) 같은 것
    // Create router: configure routing
    let app = Router::new()
        .route("/", get(handler));

    // 서버 시작: 5000 포트에서 리스닝
    // 역할: NestJS의 app.listen(5000) 같은 것
    // Start server: listen on port 5000
    // Note: Axum 0.7에서는 TcpListener를 직접 사용해야 함
    let listener = TcpListener::bind("0.0.0.0:3002")
        .await
        .unwrap();
    
    println!("Server running on http://0.0.0.0:3002");
    
    // 서버 실행: 앱을 서빙 시작
    // Run server: start serving the app
    axum::serve(listener, app)
        .await
        .unwrap();
}
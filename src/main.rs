mod handlers;
mod routes;
mod utils;

use axum::Router;

#[tokio::main]
async fn main() {
    // 构建应用路由
    let app = Router::new().merge(routes::routes());

    // 运行服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

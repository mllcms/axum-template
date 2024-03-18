use std::net::SocketAddr;

use library::tools;
use server::{config::CONFIG, router};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = router::router().await;
    tools::print_router_info(&app);

    let addr = &CONFIG.server.addr;
    tools::print_address(addr, &CONFIG.server.protocol);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

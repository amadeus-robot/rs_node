use rust::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = AmaApp::new();

    app.start().await;
    app.wait_node_inited(None);

    
}

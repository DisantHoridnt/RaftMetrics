use distributed_analytics_system::api::worker;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    worker::start_worker_node().await;
}

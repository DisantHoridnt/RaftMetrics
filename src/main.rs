use distributed_analytics_system::api::control;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    control::start_control_node().await;
}

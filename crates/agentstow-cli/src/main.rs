#[tokio::main]
async fn main() {
    let exit_code = agentstow_cli::run().await;
    std::process::exit(exit_code);
}

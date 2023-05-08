use lib::rsms::core::{Commander, Serve};
use lib::rsms::infra::log;

#[tokio::main]
async fn main() {
    log::v("rsms initializing...");
    let commander = &mut Commander::new();
    commander.init();
    commander.start();
    commander.run_loop().await;
    commander.stop();
    commander.destroy();
}

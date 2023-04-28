mod libs;

use libs::rsms::infra;
use libs::rsms::core;

fn main() {
    infra::log("good job");
    core::push();
}

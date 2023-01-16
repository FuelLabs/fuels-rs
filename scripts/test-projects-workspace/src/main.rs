//! Build all tests in tests workspace

use test_projects_workspace::build;

#[tokio::main]
async fn main() {
    build(Some(String::from("packages/fuels/tests"))).expect("failed to build workspace tests");
}

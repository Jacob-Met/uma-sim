//! uma-sim REST API binary — parity with Kotlin `SimApiServer`.

fn main() {
    let port = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8765);
    uma_sim_core::api::serve(port);
}

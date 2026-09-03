use std::path::Path;
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=../packages/uma-sim-ui/dist");
    if std::env::var("CARGO_FEATURE_EMBED_UI").is_ok() {
        let dist = Path::new("../packages/uma-sim-ui/dist");
        let index = dist.join("index.html");
        if !index.exists() {
            let _ = fs::create_dir_all(dist);
            let _ = fs::write(
                &index,
                r#"<!doctype html><html><body style="font-family:sans-serif;background:#111;color:#eee;padding:2rem">
<h1>uma-sim UI not built</h1>
<p>Run <code>cd packages/uma-sim-ui &amp;&amp; npm ci &amp;&amp; npm run build</code>, then rebuild with <code>--features embed-ui</code>.</p>
</body></html>"#,
            );
            println!("cargo:warning=packages/uma-sim-ui/dist missing; wrote placeholder index.html for embed-ui");
        }
    }
}

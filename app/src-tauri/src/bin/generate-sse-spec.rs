use std::{fs, path::Path};

use merframe_lib::event_feed::FeedEvent;
use schemars::schema_for;

#[allow(clippy::unwrap_used, clippy::expect_used)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema_for!(FeedEvent);
    let json = serde_json::to_string_pretty(&schema)?;

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .ancestors()
        .nth(2)
        .expect("crate must be two levels below workspace root");

    let output_path = workspace_dir.join("specs/sse.schema.json");

    fs::create_dir_all(output_path.parent().unwrap())?;
    fs::write(&output_path, json)?;

    println!("Generated file at {}", output_path.display());

    Ok(())
}

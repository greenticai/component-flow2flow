#![cfg(feature = "inmem-registry")]

use std::path::PathBuf;

use flow2flow_cli::{run_from_iter, testing::reset_registry};
use insta::assert_snapshot;

fn manifest_path(file: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join(file)
        .display()
        .to_string()
}

#[test]
fn validate_snapshot() {
    reset_registry();

    let weather = manifest_path("weather.yml");

    let output = run_from_iter(["f2f", "validate", &weather]).expect("validate");

    assert_snapshot!("validate_weather", output);
}

#[test]
fn publish_and_resolve_snapshots() {
    reset_registry();

    let weather = manifest_path("weather.yml");
    let weather_team = manifest_path("weather_team.yml");

    let publish_tenant =
        run_from_iter(["f2f", "publish", &weather, "--tenant", "acme"]).expect("publish tenant");
    assert_snapshot!("publish_weather_tenant", publish_tenant);

    let publish_team =
        run_from_iter(["f2f", "publish", &weather_team, "--tenant", "acme", "--team", "sales-na"])
            .expect("publish team");
    assert_snapshot!("publish_weather_team", publish_team);

    let resolve_output = run_from_iter([
        "f2f",
        "resolve",
        "assistant.weather.daily",
        "--tenant",
        "acme",
        "--team",
        "sales-na",
    ])
    .expect("resolve");

    assert_snapshot!("resolve_acme_sales_na", resolve_output);
}

#[test]
fn run_snapshot() {
    reset_registry();
    let weather = manifest_path("weather.yml");
    let input = manifest_path("weather_input.json");

    let output = run_from_iter([
        "f2f", "run", &weather, "--tenant", "acme", "--team", "sales-na", "-i", &input,
    ])
    .expect("run");

    assert_snapshot!("run_weather", output);
}

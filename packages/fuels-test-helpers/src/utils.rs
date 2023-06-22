use cargo_metadata::Metadata;
use fuel_core_chain_config::{CoinConfig, MessageConfig};
use fuels_core::types::{coin::Coin, message::Message};
use isahc::ReadResponseExt;
use serde_json::Value;
use std::env;
use tokio::process::Command;
// use toml::Value as TomlValue;

pub fn into_coin_configs(coins: Vec<Coin>) -> Vec<CoinConfig> {
    coins
        .into_iter()
        .map(Into::into)
        .collect::<Vec<CoinConfig>>()
}

pub fn into_message_configs(messages: Vec<Message>) -> Vec<MessageConfig> {
    messages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<MessageConfig>>()
}

pub async fn check_fuel_core_dependency_version() {
    // Option 2 :
    // latest_version=$(curl -s https://crates.io/api/v1/crates/fuels | grep -o '"num":"[^"]*"' | cut -d '"' -f 4 | head -n 1)
    // curl -s "https://crates.io/api/v1/crates/fuels/$latest_version/dependencies" |
    //     grep -o '"crate_id":"fuel-core","[^}]*' | head -n 1 | grep -o '"req":"[^"]*"' | cut -d '"' -f 4 | cut -c 2-

    let binary_name = "fuel-core";

    let used_fuel_version = if cfg!(not(feature = "fuel-core-lib")) {
        let path = which::which(binary_name)
            .unwrap_or_else(|_| panic!("failed to list '{binary_name}' binaries"));

        let output = Command::new(path)
            .arg("--version")
            .output()
            .await
            .expect("Failed to execute command");

        let version = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .nth(1)
            .expect("Failed to split string")
            .trim_end()
            .to_string();

        version.split('.').take(2).collect::<Vec<&str>>().join(".")
    } else {
        let mut current_dir = env::current_dir().expect("Failed to get current directory");
        current_dir.pop();
        current_dir.pop();

        let metadata: Metadata = cargo_metadata::MetadataCommand::new()
            .current_dir(current_dir)
            .exec()
            .expect("Failed to obtain cargo metadata");

        let current_package = metadata
            .packages
            .iter()
            .find(|package| package.name == "fuel-core")
            .expect("Failed to find current package");

        current_package
            .version
            .to_string()
            .split('.')
            .take(2)
            .collect::<Vec<&str>>()
            .join(".")
    };

    let crate_name = "fuels";

    let crate_info_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let mut crate_info_response =
        isahc::get(crate_info_url).expect("Failed to fetch crate information");
    let crate_info_body = crate_info_response
        .text()
        .expect("Failed to decode response body");
    let crate_info_json: Value =
        serde_json::from_str(&crate_info_body).expect("Failed to parse crate information");

    let latest_version = crate_info_json["versions"][0]["num"]
        .as_str()
        .expect("Failed to retrieve latest version");

    let dependency_info_url = format!(
        "https://crates.io/api/v1/crates/{}/{}/dependencies",
        crate_name, latest_version
    );
    let mut dependency_info_response =
        isahc::get(dependency_info_url).expect("Failed to fetch dependency information");
    let dependency_info_body = dependency_info_response
        .text()
        .expect("Failed to decode response body");
    let dependency_info_json: Value = serde_json::from_str(&dependency_info_body)
        .expect("Failed to parse dependency information");

    if let Some(dependencies) = dependency_info_json["dependencies"].as_array() {
        if let Some(fuel_core_dependency) = dependencies
            .iter()
            .find(|dependency| dependency["crate_id"].as_str() == Some("fuel-core"))
        {
            let fuel_core_req_vec = fuel_core_dependency["req"]
                .as_str()
                .expect("Failed to retrieve fuel-core requirement")
                .trim_start_matches('^')
                .to_string();

            let fuel_core_req = fuel_core_req_vec
                .split('.')
                .take(2)
                .collect::<Vec<&str>>()
                .join(".");

            if fuel_core_req != used_fuel_version {
                eprintln!("Your fuel-core version {} is not equal to the version {} used by fuels crate. This could potentially lead to errors.", used_fuel_version, fuel_core_req);
                eprintln!("Consider updating your fuel-core version to match the version used by fuels.\n");
            }
        }
    }
}

use fuel_core_chain_config::{CoinConfig, MessageConfig};
use fuels_core::types::{coin::Coin, message::Message};
use isahc::ReadResponseExt;
use serde_json::Value;
use std::env;
use tokio::process::Command;

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
        let path = match which::which(binary_name) {
            Ok(path) => path,
            Err(err) => {
                eprintln!("Failed to fetch dependency information: {}", err);
                return;
            }
        };

        let output = match Command::new(path).arg("--version").output().await {
            Ok(output) => output,
            Err(err) => {
                eprintln!("Failed to fetch dependency information: {}", err);
                return;
            }
        };

        let version = match String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .nth(1)
        {
            Some(version) => version.trim_end().to_string(),
            None => {
                eprintln!("Failed to fetch dependency information");
                return;
            }
        };

        version.split('.').take(2).collect::<Vec<&str>>().join(".")
    } else {
        let current_dir = match env::current_dir() {
            Ok(mut current_dir) => {
                current_dir.pop();
                current_dir.pop();
                current_dir
            }
            Err(err) => {
                eprintln!("Failed to fetch dependency information: {}", err);
                return;
            }
        };

        let metadata = match cargo_metadata::MetadataCommand::new()
            .current_dir(&current_dir)
            .exec()
        {
            Ok(metadata) => metadata,
            Err(err) => {
                eprintln!("Failed to fetch dependency information: {}", err);
                return;
            }
        };

        let current_package = match metadata
            .packages
            .iter()
            .find(|package| package.name == binary_name)
        {
            Some(package) => package,
            None => {
                eprintln!("Failed to fetch dependency information");
                return;
            }
        };

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
    let mut crate_info_response = match isahc::get(crate_info_url) {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Failed to fetch dependency information: {}", err);
            return;
        }
    };

    let crate_info_body = match crate_info_response.text() {
        Ok(body) => body,
        Err(err) => {
            eprintln!("Failed to fetch dependency information: {}", err);
            return;
        }
    };

    let crate_info_json: Result<Value, serde_json::error::Error> =
        serde_json::from_str(&crate_info_body);

    let latest_version = match crate_info_json {
        Ok(ref json) => match json["versions"][0]["num"].as_str() {
            Some(version) => version,
            None => {
                eprintln!("Failed to fetch dependency information");
                return;
            }
        },
        Err(err) => {
            eprintln!("Failed to fetch dependency information: {}", err);
            return;
        }
    };

    let dependency_info_url = format!(
        "https://crates.io/api/v1/crates/{}/{}/dependencies",
        crate_name, latest_version
    );

    let mut dependency_info_response = match isahc::get(dependency_info_url) {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Failed to fetch dependency information: {}", err);
            return;
        }
    };

    let dependency_info_body = match dependency_info_response.text() {
        Ok(body) => body,
        Err(err) => {
            eprintln!("Failed to fetch dependency information: {}", err);
            return;
        }
    };

    let dependency_info_json: Result<Value, serde_json::error::Error> =
        serde_json::from_str(&dependency_info_body);

    if let Ok(json) = dependency_info_json {
        if let Some(dependencies) = json["dependencies"].as_array() {
            if let Some(fuel_core_dependency) = dependencies
                .iter()
                .find(|dependency| dependency["crate_id"].as_str() == Some("fuel-core"))
            {
                let fuel_core_req_vec = match fuel_core_dependency["req"].as_str() {
                    Some(req) => req.trim_start_matches('^').to_string(),
                    None => {
                        eprintln!("Failed to retrieve fuel-core requirement");
                        return;
                    }
                };

                let fuel_core_req = fuel_core_req_vec
                    .split('.')
                    .take(2)
                    .collect::<Vec<&str>>()
                    .join(".-");

                if fuel_core_req != used_fuel_version {
                    eprintln!(
                        "Your fuel-core version {} is not equal to the version {} used by fuels crate. This could potentially lead to errors.",
                        used_fuel_version, fuel_core_req
                    );
                    eprintln!("Consider updating your fuel-core version to match the version used by fuels.\n");
                }
            }
        }
    } else {
        eprintln!(
            "Failed to parse dependency information: {:?}",
            dependency_info_json.err()
        );
    }
}

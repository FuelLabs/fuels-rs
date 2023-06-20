use fuel_core_chain_config::{CoinConfig, MessageConfig};
use fuels_core::types::{coin::Coin, message::Message};
use isahc::ReadResponseExt;
use serde_json::Value;

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

pub async fn get_fuel_core_dependency_version() -> String { let crate_name = "fuels";
    let crate_name = "fuels";

    let crate_info_url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let mut crate_info_response = isahc::get(crate_info_url).expect("Failed to fetch crate information");
    let crate_info_body = crate_info_response.text().expect("Failed to decode response body");
    let crate_info_json: Value = serde_json::from_str(&crate_info_body).expect("Failed to parse crate information");

    let latest_version = crate_info_json["versions"][0]["num"]
        .as_str()
        .expect("Failed to retrieve latest version");

    let dependency_info_url = format!("https://crates.io/api/v1/crates/{}/{}/dependencies", crate_name, latest_version);
    let mut dependency_info_response = isahc::get(dependency_info_url).expect("Failed to fetch dependency information");
    let dependency_info_body = dependency_info_response.text().expect("Failed to decode response body");
    let dependency_info_json: Value = serde_json::from_str(&dependency_info_body).expect("Failed to parse dependency information");

    if let Some(dependencies) = dependency_info_json["dependencies"].as_array() {
        if let Some(fuel_core_dependency) = dependencies.iter().find(|dependency| {
            dependency["crate_id"].as_str() == Some("fuel-core")
        }) {
            let fuel_core_req = fuel_core_dependency["req"]
                .as_str()
                .expect("Failed to retrieve fuel-core requirement")
                .trim_start_matches('^')
                .to_string();

            return fuel_core_req;
        }
    }

    eprintln!("Fuel-core dependency not found");
    String::new()
}
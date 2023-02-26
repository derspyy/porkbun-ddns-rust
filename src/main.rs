use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use toml;
use ureq::Agent;

const ENDPOINT: &str = "https://porkbun.com/api/json/v3";

fn main() {
    let config_path = Path::new("config.toml");
    let config: Config;
    match config_path.exists() {
        true => {
            let data = fs::read_to_string(&config_path).unwrap();
            config = toml::from_str(&data).unwrap();
        }
        false => {
            config = Config::default();
            let data = toml::to_string_pretty(&config).unwrap();
            fs::write(&config_path, &data).unwrap();
            return;
        }
    }

    let agent = Agent::new();

    let ip: String;
    match config.ip.address.is_empty() {
        true => {
            let ping_endpoint = format!("{}/ping", ENDPOINT);
            let ping_response: Value = agent
                .post(&ping_endpoint)
                .send_json(&config.keys)
                .unwrap()
                .into_json()
                .unwrap();
            if let Some(x) = ping_response["yourIp"].as_str() {
                ip = String::from(x);
            } else {
                println!("Couldn't retrieve IP address.");
                return;
            }
        }
        false => {
            ip = config.ip.address;
        }
    }

    let full_domain: String;
    match config.domain.subdomain.is_empty() {
        true => {
            full_domain = config.domain.base.clone();
        }
        false => {
            full_domain = format!("{}.{}", config.domain.subdomain, &config.domain.base);
        }
    }

    let record_type = match config.ip.ipv6 {
        true => "AAAA",
        false => "A",
    };

    let mut record = None;
    let mut ttl = None;
    let mut prio = None;

    let records_endpoint = format!("{}/dns/retrieve/{}", ENDPOINT, &config.domain.base);
    let records_data = agent
        .post(&records_endpoint)
        .send_json(&config.keys)
        .unwrap()
        .into_string()
        .unwrap();
    let records_response: RecordsResponse = serde_json::from_str(&records_data).unwrap();
    if records_response.status.as_str() != "SUCCESS" {
        println!("Couldn't retrieve records");
        return;
    }
    for x in records_response.records {
        if x.name.as_str() == full_domain {
            record = Some(x);
            break
        }
    }

    if let Some(x) = record {
        let delete_endpoint = format!("{}/dns/delete/{}/{}", ENDPOINT, &config.domain.base, x.id);
        let delete_response: Value = agent
            .post(&delete_endpoint)
            .send_json(&config.keys)
            .unwrap()
            .into_json()
            .unwrap();
        if let Some(_) = delete_response["status"].as_str() {
            println!("Deleting existing {} record", &record_type);
        } else {
            println!("Couldn't delete record.");
            return;
        }
        ttl = Some(x.ttl);
        prio = Some(x.prio);
    } else {
        println!("No record to be deleted.")
    }

    let create_endpoint = format!("{}/dns/create/{}", ENDPOINT, &config.domain.base);
    let create_body = CreateRecord {
        secretapikey: config.keys.secretapikey,
        apikey: config.keys.apikey,
        name: full_domain.clone(),
        _type: String::from(record_type),
        content: String::from(&ip),
        ttl: ttl,
        prio: prio,
    };
    let create_response: Value = agent
        .post(&create_endpoint)
        .send_json(&create_body)
        .unwrap()
        .into_json()
        .unwrap();
    match create_response["status"].as_str() {
        Some("SUCCESS") => {
            println!("Creating record: {} with answer of {}", &full_domain, &ip)
        }
        _ => {
            println!("Couldn't create record.")
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    keys: Keys,
    domain: Domain,
    ip: Ip,
}

#[derive(Serialize, Deserialize, Default)]
struct Keys {
    secretapikey: String,
    apikey: String,
}
#[derive(Serialize, Deserialize, Default)]
struct Domain {
    subdomain: String,
    base: String,
}
#[derive(Serialize, Deserialize, Default)]
struct Ip {
    address: String,
    ipv6: bool,
}

#[derive(Deserialize)]
struct RecordsResponse {
    status: String,
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    id: String,
    name: String,
    #[serde(rename = "type")]
    _type: String,
    ttl: String,
    prio: String,
}

#[derive(Serialize)]
struct CreateRecord {
    secretapikey: String,
    apikey: String,
    name: String,
    #[serde(rename = "type")]
    _type: String,
    content: String,
    ttl: Option<String>,
    prio: Option<String>,
}
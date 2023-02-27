use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

use ureq::Agent;

const ENDPOINT: &str = "https://porkbun.com/api/json/v3";
const ENDPOINT_IPV4: &str = "https://api-ipv4.porkbun.com/api/json/v3";

fn main() {
    let config_path = Path::new("config.toml");
    let config: Config = match config_path.exists() {
        true => {
            let data = fs::read_to_string(config_path).unwrap();
            toml::from_str(&data).unwrap()
        }
        false => {
            let data = toml::to_string_pretty(&Config::default()).unwrap();
            fs::write(config_path, data).unwrap();
            return;
        }
    };

    let agent = Agent::new();

    let ip: String;
    match config.ip.address.is_empty() {
        true => {
            let ping_endpoint = match config.ip.ipv6 {
                true => format!("{}/ping", ENDPOINT),
                false => format!("{}/ping", ENDPOINT_IPV4),
            };
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

    let full_domain = match config.domain.subdomain.is_empty() {
        true => {
            config.domain.base.clone()
        }
        false => {
            format!("{}.{}", config.domain.subdomain, &config.domain.base)
        }
    };

    let record_type = match config.ip.ipv6 {
        true => "AAAA",
        false => "A",
    };

    let mut record = None;
    let mut ttl = None;
    let mut prio = None;
    let mut notes = None;

    let records_endpoint = format!("{}/dns/retrieve/{}", ENDPOINT, &config.domain.base);
    let records_response: RecordsResponse = agent
        .post(&records_endpoint)
        .send_json(&config.keys)
        .unwrap()
        .into_json()
        .unwrap();
    if records_response.status.as_str() != "SUCCESS" {
        println!("Couldn't retrieve records");
        return;
    }
    for x in records_response.records {
        if x.name.as_str() == full_domain {
            record = Some(x);
            break;
        }
    }

    if let Some(x) = record {
        if x.content == ip {
            println!(
                "Existing {} record already matches answer {}",
                record_type, &ip
            );
            return;
        }
        let delete_endpoint = format!("{}/dns/delete/{}/{}", ENDPOINT, &config.domain.base, x.id);
        let delete_response: Value = agent
            .post(&delete_endpoint)
            .send_json(&config.keys)
            .unwrap()
            .into_json()
            .unwrap();
        if delete_response["status"].as_str().is_some() {
            println!("Deleting existing {} record", &record_type);
        } else {
            println!("Couldn't delete record.");
            return;
        }
        ttl = x.ttl;
        prio = x.prio;
        notes = x.notes;
    } else {
        println!("No record to be deleted.")
    }

    let create_endpoint = format!("{}/dns/create/{}", ENDPOINT, &config.domain.base);
    let create_body = CreateRecord {
        secretapikey: config.keys.secretapikey,
        apikey: config.keys.apikey,
        name: config.domain.subdomain,
        _type: String::from(record_type),
        content: String::from(&ip),
        ttl,
        prio,
        notes,
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
    content: String,
    ttl: Option<String>,
    prio: Option<String>,
    notes: Option<String>,
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
    notes: Option<String>,
}

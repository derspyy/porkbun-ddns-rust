use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::{env, fs};

use ureq::Agent;

const ENDPOINT: &str = "https://api.porkbun.com/api/json/v3";
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

    let keys = match config.keys {
        Some(x) => x,
        None => get_env().expect("Couldn't get environment variables"),
    };

    let ip: String;
    let endpoint: &str = match config.ip.ipv6 {
        true => ENDPOINT,
        false => ENDPOINT_IPV4,
    };
    match config.ip.address.is_empty() {
        true => {
            let ping_response: Value = agent
                .post(&format!("{}/ping", endpoint))
                .send_json(&keys)
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
        true => config.domain.base.clone(),
        false => {
            format!("{}.{}", config.domain.subdomain, &config.domain.base)
        }
    };

    let record_type = match config.ip.ipv6 {
        true => "AAAA",
        false => "A",
    };

    let mut ttl = None;
    let mut prio = None;
    let mut notes = None;

    let records_endpoint = format!(
        "{}/dns/retrieveByNameType/{}/{}/{}",
        endpoint, &config.domain.base, record_type, &config.domain.subdomain
    );
    let records_response: RecordsResponse = agent
        .post(&records_endpoint)
        .send_json(&keys)
        .unwrap()
        .into_json()
        .unwrap();
    if records_response.status.as_str() != "SUCCESS" {
        println!("Couldn't retrieve records");
        return;
    }

    let record = records_response.records.get(0);

    if let Some(x) = record {
        if x.content == ip {
            println!(
                "Existing {} record already matches answer {}",
                record_type, &ip
            );
            return;
        }
        let delete_endpoint = format!("{}/dns/delete/{}/{}", endpoint, &config.domain.base, x.id);
        let delete_response: Value = agent
            .post(&delete_endpoint)
            .send_json(&keys)
            .unwrap()
            .into_json()
            .unwrap();
        if delete_response["status"].as_str().is_some() {
            println!("Deleting existing {} record", &record_type);
        } else {
            println!("Couldn't delete record.");
            return;
        }
        ttl = x.ttl.clone();
        prio = x.prio.clone();
        notes = x.notes.clone();
    } else {
        println!("No record to be deleted.")
    }

    let create_endpoint = format!("{}/dns/create/{}", endpoint, &config.domain.base);
    let create_body = CreateRecord {
        secretapikey: keys.secretapikey,
        apikey: keys.apikey,
        name: config.domain.subdomain,
        _type: String::from(record_type),
        content: String::from(&ip),
        ttl,
        prio,
        notes,
    };
    let create_response: Value = agent
        .post(&create_endpoint)
        .send_json(create_body)
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

fn get_env() -> Result<Keys, env::VarError> {
    Ok(Keys {
        secretapikey: env::var("PORKBUN_SECRET_API_KEY")?,
        apikey: env::var("PORKBUN_API_KEY")?,
    })
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    keys: Option<Keys>,
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

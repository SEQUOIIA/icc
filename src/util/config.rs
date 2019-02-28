use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

// Config
#[derive(Deserialize, Serialize)]
pub struct Config {
    // Address + port for web interface, e.g. "0.0.0.0:4017"
    pub bind_address: Option<String>,
    // An array of addresses to use when monitoring network connectivity, e.g. ["8.8.8.8", "1.1.1.1"]
    pub addresses_to_monitor: Option<Vec<String>>,
    // Maximum ping timeouts before it counts as "downtime"
    pub max_timeouts: Option<u32>,
    // Max time waiting for a singular ping, before deeming it a timeout.
    pub max_ping_timeout: Option<u64>,
    // Local database file
    pub db: Option<String>,
    // If set, logs downtimes in clear text at the specified path
    pub clear_text_log: Option<String>
}

pub fn config() -> Config {
    let mut save_to_file  : bool = false;
    let mut config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("config.toml").unwrap();
    let mut buf = Vec::new();
    config_file.read_to_end(&mut buf);
    let mut config : Config = toml::from_slice(&buf).unwrap();

    if let None = config.bind_address {
        config.bind_address = Some("0.0.0.0:4017".to_owned());
        save_to_file = true;
    }

    if let None = config.addresses_to_monitor {
        config.addresses_to_monitor = Some(vec!("8.8.8.8".to_owned(), "1.1.1.1".to_owned()));
        save_to_file = true;
    }

    if let None = config.max_timeouts {
        config.max_timeouts = Some(3);
    }

    if let None = config.max_ping_timeout {
        config.max_ping_timeout = Some(1000);
    }

    if let None = config.db {
        let rand_filename : String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(17)
            .collect();
        config.db = Some(rand_filename);
        save_to_file = true;
    }

    if save_to_file {
        let payload = toml::to_string_pretty(&config).unwrap();

        let mut config_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open("config.toml").unwrap();

        config_file.write_all(payload.as_bytes());
    }

    config
}
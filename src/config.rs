/*
 * Copyright 2019 Tamas Blummer
 * Copyright 2020 BDK Team
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use crate::error::Error;

use bitcoin::Network;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub encryptedwalletkey: String,
    pub keyroot: String,
    pub lookahead: u32,
    pub birth: u64,
    pub network: Network,
    pub bitcoin_peers: Vec<SocketAddr>,
    pub bitcoin_connections: usize,
    pub bitcoin_discovery: bool,
}

impl Config {
    pub fn new(encryptedwalletkey: &str, keyroot: &str, lookahead: u32, birth: u64, network: Network) -> Config {
        Config {
            encryptedwalletkey: String::from(encryptedwalletkey),
            keyroot: String::from(keyroot),
            lookahead,
            birth,
            network,
            bitcoin_peers: vec![],
            bitcoin_connections: 0,
            bitcoin_discovery: false,
        }
    }

    pub fn update(&self, bitcoin_peers: Vec<SocketAddr>, bitcoin_connections: usize, bitcoin_discovery: bool) -> Config {
        Config {
            encryptedwalletkey: self.encryptedwalletkey.clone(),
            keyroot: self.keyroot.clone(),
            lookahead: self.lookahead,
            birth: self.birth,
            network: self.network,
            bitcoin_peers,
            bitcoin_connections,
            bitcoin_discovery,
        }
    }
}

pub fn save(config_path: &Path, file_path: &Path, config: &Config) -> Result<(), Error> {
    fs::create_dir_all(&config_path)?;
    let mut file = File::create(file_path)?;
    let config_string = toml::to_string(config).unwrap();

    file.write_all(config_string.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

pub fn load(file_path: &Path) -> Result<Config, Error> {
    // get config (if any)
    let mut file = File::open(file_path)?;
    let mut config_string = String::new();
    file.read_to_string(&mut config_string)?;
    match toml::from_str(config_string.as_str()) {
        Ok(c) => Ok(c),
        Err(e) => Err(e.into())
    }
}

pub fn remove(config_path: &Path) -> Result<(), Error> {
     match fs::remove_dir_all(config_path) {
         Ok(()) => Ok(()),
         Err(e) => Err(e.into())
     }
}

#[cfg(test)]
mod test {
    use std::{fs, io};
    use std::error::Error;
    use std::path::PathBuf;
    use std::str::FromStr;

    use bitcoin::Network;

    use crate::config;
    use crate::config::Config;

    #[test]
    fn save_load_delete() {
        let test_config = Config::new(
            "encryptedwalletkey",
            "keyroot",
            0, 0, Network::Testnet);

        let workdir_path = PathBuf::from("./test1");
        let mut config_path = workdir_path.clone();
        config_path.push(test_config.network.to_string());
        let mut file_path = config_path.clone();
        file_path.push("bdk.cfg");

        assert_eq!(config::save(&config_path, &file_path, &test_config).is_ok(), true);
        let loaded = config::load(&file_path);
        assert_eq!(loaded.is_ok(), true);
        assert_eq!(loaded.unwrap(), test_config);
        assert_eq!(config::remove(&workdir_path).is_ok(), true);
    }

    #[test]
    fn save_update_load_delete() {
        let test_config = Config::new(
            "encryptedwalletkey",
            "keyroot",
            0, 0, Network::Testnet);

        let workdir_path = PathBuf::from("./test2");
        let mut config_path = workdir_path.clone();
        config_path.push(test_config.network.to_string());
        let mut file_path = config_path.clone();
        file_path.push("bdk.cfg");

        assert_eq!(config::save(&config_path, &file_path, &test_config).is_ok(), true);

        let loaded = config::load(&file_path);
        assert_eq!(loaded.is_ok(), true);
        let loaded = loaded.unwrap();
        assert_eq!(loaded, test_config);

        let bitcoin_peers = vec! {"127.0.0.1:8080".parse().unwrap(), "127.0.0.1:8081".parse().unwrap(), "127.0.0.1:8082".parse().unwrap()};
        let updated = loaded.update(bitcoin_peers, 10, false);
        let saved_updated = config::save(&config_path, &file_path, &updated);
        assert_eq!(saved_updated.is_ok(), true);

        let loaded_updated = config::load(&file_path);
        assert_eq!(loaded_updated.is_ok(), true);

        let loaded_updated = loaded_updated.unwrap();
        assert_eq!(updated, loaded_updated);
        assert_eq!(loaded_updated.bitcoin_peers.len(), 3);
        assert_eq!(loaded_updated.bitcoin_connections, 10);
        assert_eq!(loaded_updated.bitcoin_discovery, false);

        assert_eq!(config::remove(&workdir_path).is_ok(), true);
        let loaded_updated = config::load(&file_path);
        assert_eq!(loaded_updated.is_ok(), false);
    }
}


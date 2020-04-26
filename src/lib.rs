/*
 * Copyright 2019 Tamas Blummer
 * Copyright 2020 BTCDK Team
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

#![allow(non_snake_case)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use std::path::PathBuf;

use bitcoin::Network;

use crate::config::Config;
use crate::error::Error;
use crate::wallet::{Wallet, KEY_LOOK_AHEAD};
use std::fs;
use std::net::SocketAddr;

mod blockdownload;
mod config;
mod db;
mod error;
mod jni;
mod p2p_bitcoin;
mod sendtx;
mod store;
mod trunk;
mod wallet;

const CONFIG_FILE_NAME: &str = "btcdk.cfg";

// load config

pub fn load_config(work_dir: PathBuf, network: Network) -> Result<Config, Error> {
    let mut file_path = PathBuf::from(work_dir);
    file_path.push(network.to_string());
    file_path.push(CONFIG_FILE_NAME);

    config::load(&file_path)
}

// remove config

pub fn remove_config(work_dir: PathBuf, network: Network) -> Result<Config, Error> {
    let mut config_path = PathBuf::from(work_dir);
    config_path.push(network.to_string());
    let mut file_path = config_path.clone();
    file_path.push(CONFIG_FILE_NAME);

    let config = config::load(&file_path)?;
    config::remove(&config_path)?;
    Ok(config)
}

// update config

pub fn update_config(work_dir: PathBuf, network: Network, bitcoin_peers: Vec<SocketAddr>,
                     bitcoin_connections: usize, bitcoin_discovery: bool) -> Result<Config, Error> {
    let mut config_path = PathBuf::from(work_dir);
    config_path.push(network.to_string());
    let mut file_path = config_path.clone();
    file_path.push(CONFIG_FILE_NAME);

    let config = config::load(&file_path)?;
    let updated_config = config.update(bitcoin_peers, bitcoin_connections, bitcoin_discovery);
    config::save(&config_path, &file_path, &updated_config)?;
    Ok(updated_config)
}

// init config

pub struct InitResult {
    mnemonic_words: String,
    deposit_address: String,
}

impl InitResult {
    fn new(mnemonic_words: String, deposit_address: String) -> InitResult {
        InitResult {
            mnemonic_words,
            deposit_address,
        }
    }
}

pub fn init_config(work_dir: PathBuf, network: Network, passphrase: &str, pd_passphrase: Option<&str>) -> Result<Option<InitResult>, Error> {
    let mut config_path = PathBuf::from(work_dir);
    config_path.push(network.to_string());
    fs::create_dir_all(&config_path).expect(format!("unable to create config_path: {}", &config_path.to_str().unwrap()).as_str());

    let mut file_path = config_path.clone();
    file_path.push(CONFIG_FILE_NAME);

    if let Ok(_config) = config::load(&file_path) {
        // do not init if a config already exists, return none
        Ok(Option::None)
    } else {
        // create new wallet
        let (mnemonic_words, deposit_address, wallet) = Wallet::new(network, passphrase, pd_passphrase);
        let mnemonic_words = mnemonic_words.to_string();
        let deposit_address = deposit_address.to_string();

        let encryptedwalletkey = hex::encode(wallet.encrypted().as_slice());
        let keyroot = wallet.master_public().to_string();
        let lookahead = KEY_LOOK_AHEAD;
        let birth = wallet.birth();

        // init database
        db::init(&config_path, &wallet.coins, &wallet.master);

        // save config
        let config = Config::new(encryptedwalletkey.as_str(),
                                 keyroot.as_str(), lookahead, birth, network);
        config::save(&config_path, &file_path, &config)?;

        Ok(Option::from(InitResult::new(mnemonic_words, deposit_address)))
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use crate::{init_config, load_config, update_config, remove_config};
    use bitcoin::Network;
    use std::net::SocketAddr;
    use std::str::FromStr;

    const PASSPHRASE: &str = "correct horse battery staple";
    const PD_PASSPHRASE_1: &str = "test123";

    #[test]
    fn init_update_remove_config() {
        let work_dir: PathBuf = PathBuf::from(".");

        let inited = init_config(work_dir.clone(), Network::Regtest,
                                 PASSPHRASE, Some(PD_PASSPHRASE_1)).unwrap();
        let peer1 = SocketAddr::from_str("127.0.0.1:18333").unwrap();
        let peer2  = SocketAddr::from_str("10.0.0.10:18333").unwrap();
        let updated = update_config(work_dir.clone(), Network::Regtest,
                                    vec!(peer1, peer2),
                                    3, true).unwrap();
        let removed = remove_config(work_dir, Network::Regtest).unwrap();
        assert_eq!(removed.network, Network::Regtest);
        assert_eq!(removed.bitcoin_peers.len(), 2);
        assert_eq!(removed.bitcoin_peers[0], peer1);
        assert_eq!(removed.bitcoin_peers[1], peer2);
        assert_eq!(removed.bitcoin_connections, 3);
        assert!(removed.bitcoin_discovery);
    }
}
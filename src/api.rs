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

use std::{fs, time};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use bitcoin::{Address, BitcoinHash, Network};
use bitcoin::hashes::core::str::FromStr;
use bitcoin::util::bip32::ExtendedPubKey;
use bitcoin_hashes::sha256d;
use bitcoin_wallet::account::MasterAccount;
use futures::{executor::ThreadPoolBuilder};
use futures_timer::Delay;
use log::{info, warn};
use log::{debug, error};
use murmel::chaindb::ChainDB;
use once_cell::sync::Lazy;

use crate::{config, db};
use crate::config::Config;
use crate::db::DB;
use crate::error::Error;
use crate::p2p_bitcoin::{ChainDBTrunk, P2PBitcoin};
use crate::store::{ContentStore, SharedContentStore};
use crate::trunk::Trunk;
use crate::wallet::{KEY_LOOK_AHEAD, Wallet};

const CONFIG_FILE_NAME: &str = "bdk.cfg";

static CONTENT_STORE: Lazy<Arc<RwLock<Option<SharedContentStore>>>> = Lazy::new(|| Arc::new(RwLock::new(None::<SharedContentStore>)));

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
    pub mnemonic_words: String,
    pub deposit_address: Address,
}

impl InitResult {
    fn new(mnemonic_words: String, deposit_address: Address) -> InitResult {
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
        let deposit_address = deposit_address;

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

pub fn start(work_dir: PathBuf, network: Network, rescan: bool) -> Result<(), Error> {
    let p2p_bitcoin;
    let content_store;

    match CONTENT_STORE.write() {
        Err(e) => {
            error!("{:?}", e);
            return Ok(());
        }
        Ok(mut cs) => {
            if cs.is_some() {
                debug!("content store exists");
                return Ok(());
            } else {
                debug!("content store not initialized");

                let mut config_path = PathBuf::from(work_dir);
                config_path.push(network.to_string());

                let mut config_file_path = config_path.clone();
                config_file_path.push(CONFIG_FILE_NAME);

                info!("config file path: {}", &config_file_path.to_str().unwrap());
                let config = config::load(&config_file_path).expect("can not open config file");

                let mut chain_file_path = config_path.clone();
                chain_file_path.push("bdk.chain");

                let mut chain_db = ChainDB::new(chain_file_path.as_path(), network).expect("can not open chain db");
                chain_db.init().expect("can not initialize db");
                let chain_db = Arc::new(RwLock::new(chain_db));

                let db = open_db(&config_path);
                let db = Arc::new(Mutex::new(db));

                // get master account
                let mut bitcoin_wallet;
                let mut master_account = MasterAccount::from_encrypted(
                    hex::decode(config.encryptedwalletkey).expect("encryptedwalletkey is not hex").as_slice(),
                    ExtendedPubKey::from_str(config.keyroot.as_str()).expect("keyroot is malformed"),
                    config.birth,
                );

                // load wallet from master account
                {
                    let mut db = db.lock().unwrap();
                    let mut tx = db.transaction();
                    let account = tx.read_account(0, 0, network, config.lookahead).expect("can not read account 0/0");
                    master_account.add_account(account);
                    let account = tx.read_account(0, 1, network, config.lookahead).expect("can not read account 0/1");
                    master_account.add_account(account);
                    let account = tx.read_account(1, 0, network, 0).expect("can not read account 1/0");
                    master_account.add_account(account);
                    let coins = tx.read_coins(&mut master_account).expect("can not read coins");
                    bitcoin_wallet = Wallet::from_storage(coins, master_account);
                }

                // rescan chain if requested
                if rescan {
                    let chain_db = chain_db.read().unwrap();
                    let mut after = None;
                    for cached_header in chain_db.iter_trunk_rev(None) {
                        if (cached_header.stored.header.time as u64) < config.birth {
                            after = Some(cached_header.bitcoin_hash());
                            break;
                        }
                    }
                    if let Some(after) = after {
                        info!("Re-scanning after block {}", &after);
                        let mut db = db.lock().unwrap();
                        let mut tx = db.transaction();
                        tx.rescan(&after).expect("can not re-scan");
                        tx.commit();
                        bitcoin_wallet.rescan();
                    }
                }

                let trunk = Arc::new(ChainDBTrunk { chaindb: chain_db.clone() });
                info!("Wallet balance: {} satoshis {} available", bitcoin_wallet.balance(), bitcoin_wallet.available_balance(trunk.len(), |h| trunk.get_height(h)));

                content_store =
                    Arc::new(RwLock::new(
                        ContentStore::new(db.clone(), trunk, bitcoin_wallet).expect("can not initialize content store")));

                *cs = Option::Some(content_store.clone());

                p2p_bitcoin = P2PBitcoin::new(config.network, config.bitcoin_connections, config.bitcoin_peers, config.bitcoin_discovery, chain_db.clone(), db.clone(),
                                              content_store.clone(), config.birth);
            }
        }
    }

    let mut thread_pool = ThreadPoolBuilder::new().name_prefix("futures ").create().expect("can not start thread pool");
    p2p_bitcoin.start(&mut thread_pool);
    thread_pool.run(check_stopped(content_store));

    {
        let mut cs = CONTENT_STORE.write().unwrap();
        *cs = Option::None;
        debug!("content store set to None");
        p2p_bitcoin.shutdown()
    }
    Ok(())
}

async fn check_stopped(store: Arc<RwLock<ContentStore>>) -> () {
    info!("start check_stopped");
    let mut stopped = false;
    while !stopped {
        Delay::new(time::Duration::from_millis(100)).await.unwrap();
        stopped = store.read().unwrap().get_stopped();
    }
    warn!("stopped");
}

pub fn stop() -> () {
    info!("stopping");
    let store = CONTENT_STORE.read().unwrap().as_ref().unwrap().clone();
    store.write().unwrap().set_stopped(true);
}

#[derive(Debug, Clone)]
pub struct BalanceAmt { pub balance: u64, pub confirmed: u64 }

impl BalanceAmt {
    fn new(balance: u64, confirmed: u64) -> BalanceAmt {
        BalanceAmt { balance, confirmed }
    }
}

pub fn balance() -> Result<BalanceAmt, Error> {
    let store = CONTENT_STORE.read().unwrap().as_ref().unwrap().clone();
    let bal_vec = store.read().unwrap().balance();
    Ok(BalanceAmt::new(bal_vec[0], bal_vec[1]))
}

pub fn deposit_addr() -> Address {
    let store = CONTENT_STORE.read().unwrap().as_ref().unwrap().clone();
    let addr = store.write().unwrap().deposit_address();
    addr
}

#[derive(Debug, Clone)]
pub struct WithdrawTx { pub txid: sha256d::Hash, pub fee: u64 }

impl WithdrawTx {
    fn new(txid: sha256d::Hash, fee: u64) -> WithdrawTx {
        WithdrawTx { txid, fee }
    }
}

pub fn withdraw(passphrase: String, address: Address, fee_per_vbyte: u64, amount: Option<u64>) -> Result<WithdrawTx, Error> {
    let store = CONTENT_STORE.read().unwrap().as_ref().unwrap().clone();
    let withdraw = store.write().unwrap().withdraw(passphrase, address, fee_per_vbyte, amount);
    match withdraw {
        Ok((t, f)) => {
            Ok(WithdrawTx::new(t.txid(), f))
        }
        Err(e) => {
            Err(e)
        }
    }
}

fn open_db(config_path: &Path) -> DB {
    let mut db_path = PathBuf::from(config_path);
    const DB_FILE_NAME: &str = "bdk.db";
    db_path.push(DB_FILE_NAME);
    let db = DB::new(db_path.as_path()).expect(format!("Can't open DB {}", db_path.to_str().expect("can't get db_path")).as_str());
    db
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    use bitcoin::Network;
    use env_logger::Env;
    use log::info;

    use crate::api::{init_config, remove_config, start, stop, update_config};

    const PASSPHRASE: &str = "correct horse battery staple";
    const PD_PASSPHRASE_1: &str = "test123";

    #[test]
    fn init_update_remove_config() {
        env_logger::from_env(Env::default().default_filter_or("info")).init();
        info!("TEST init_update_remove_config()");

        let work_dir: PathBuf = PathBuf::from(".");

        let inited = init_config(work_dir.clone(), Network::Regtest,
                                 PASSPHRASE, Some(PD_PASSPHRASE_1)).unwrap();
        let peer1 = SocketAddr::from_str("127.0.0.1:18333").unwrap();
        let peer2 = SocketAddr::from_str("10.0.0.10:18333").unwrap();
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

    #[test]
    fn init_start_stop_remove_config() {
        env_logger::from_env(Env::default().default_filter_or("info")).init();
        info!("TEST init_start_stop_remove_config()");

        let work_dir: PathBuf = PathBuf::from(".");

        let inited = init_config(work_dir.clone(), Network::Regtest,
                                 PASSPHRASE, Some(PD_PASSPHRASE_1)).unwrap();

        thread::spawn(move || {
            info!("start 1");
            let work_dir: PathBuf = PathBuf::from(".");
            assert_eq!(start(work_dir, Network::Regtest, false).unwrap(), ());
            info!("start 1 returned")
        });

        thread::spawn(move || {
            info!("start 2");
            let work_dir: PathBuf = PathBuf::from(".");
            assert_eq!(start(work_dir, Network::Regtest, false).unwrap(), ());
            debug!("start 2 returned")
        });

        thread::sleep(Duration::from_millis(1000));
        assert_eq!(stop(), ());
        info!("stop returned");

        remove_config(work_dir.clone(), Network::Regtest).unwrap();
        info!("removed config");
    }
}
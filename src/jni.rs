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

use std::convert::TryFrom;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

use bitcoin::{BitcoinHash, Network};
use bitcoin::util::bip32::ExtendedPubKey;
use bitcoin_wallet::account::MasterAccount;

use crate::p2p_bitcoin::{ChainDBTrunk, P2PBitcoin};
use crate::store::ContentStore;
use crate::trunk::Trunk;
use crate::wallet::Wallet;
use futures::{
    executor::ThreadPoolBuilder,
    future,
};
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use jni::sys::{jboolean, jint, jobject, jobjectArray};
use murmel::chaindb::ChainDB;

use crate::config::Config;
use crate::{config, wallet, db};
use crate::wallet::KEY_LOOK_AHEAD;

// Optional<Config> org.btcdk.jni.BtcDkLib.loadConfig(String workDir, int network)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_loadConfig(env: JNIEnv, _: JObject,
                                                                    j_workdir: JString,
                                                                    j_network: jint) -> jobject {
    let workdir = string_from_jstring(&env, j_workdir);
    let network = network_from_jint(j_network);

    let mut file_path = PathBuf::from(workdir);
    file_path.push(network.to_string());
    file_path.push("btcdk.cfg");

    if let Ok(config) = config::load(&file_path) {
        j_optional_config(&env, &config)
    } else {
        j_optional_empty(&env)
    }
}

// Optional<Config> org.btcdk.jni.BtcDkLib.removeConfig(String workDir, int network)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_removeConfig(env: JNIEnv, _: JObject,
                                                                      j_workdir: JString,
                                                                      j_network: jint) -> jobject {
    let workdir = string_from_jstring(&env, j_workdir);
    let network = network_from_jint(j_network);

    let mut config_path = PathBuf::from(workdir);
    config_path.push(network.to_string());
    let mut file_path = config_path.clone();
    file_path.push("btcdk.cfg");

    if let Ok(config) = config::load(&file_path) {
        config::remove(&config_path).expect("error deleting config");
        j_optional_config(&env, &config)
    } else {
        j_optional_empty(&env)
    }
}

// Optional<Config> org.btcdk.jni.BtcDkLib.updateConfig(String workDir, int network, String[] bitcoinPeers, int bitcoinConnections, boolean bitcoinDiscovery)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_updateConfig(env: JNIEnv, _: JObject,
                                                                      j_workdir: JString,
                                                                      j_network: jint,
                                                                      j_bitcoin_peers: jobjectArray,
                                                                      j_bitcoin_connections: jint,
                                                                      j_bitcoin_discovery: jboolean) -> jobject {
    let workdir = string_from_jstring(&env, j_workdir);
    let network = network_from_jint(j_network);

    let mut config_path = PathBuf::from(workdir);
    config_path.push(network.to_string());
    let mut file_path = config_path.clone();
    file_path.push("btcdk.cfg");

    let bitcoin_peers_length = env.get_array_length(j_bitcoin_peers)
        .expect("error get_array_length j_bitcoin_peers");

    let mut bitcoin_peers: Vec<SocketAddr> = Vec::new();

    for i in 0..(bitcoin_peers_length) {
        let bitcoin_peer = env.get_object_array_element(j_bitcoin_peers, i)
            .expect("error get_object_array_element j_bitcoin_peers");
        let bitcoin_peer = JString::try_from(bitcoin_peer)
            .expect("error JString::try_from j_bitcoin_peers element");
        let bitcoin_peer = env.get_string(bitcoin_peer)
            .expect("error env.get_string bitcoin_peer");
        let bitcoin_peer = bitcoin_peer.to_str()
            .expect("error bitcoin_peer.toStr()");

        let bitcoin_peer_addr = SocketAddr::from_str(bitcoin_peer)
            .expect("error SocketAddr::from_str(bitcoin_peer)");

        let index = usize::try_from(i).expect("usize::try_from(bitcoin_peers_length");
        bitcoin_peers[index] = bitcoin_peer_addr;
    }

    let bitcoin_connections = usize::try_from(j_bitcoin_connections).expect("usize::try_from(j_bitcoin_connections");
    let bitcoin_discovery = j_bitcoin_discovery == 1;

    if let Ok(config) = config::load(&file_path) {
        let updated_config = config.update(bitcoin_peers, bitcoin_connections, bitcoin_discovery);
        config::save(&config_path, &file_path, &updated_config).expect("error saving updated_config");
        j_optional_config(&env, &updated_config)
    } else {
        j_optional_empty(&env)
    }
}

// Optional<InitResult> org.btcdk.jni.BtcDkLib.initConfig(String workDir, int network, String passphrase, String pdPassphrase)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_initConfig(env: JNIEnv, _: JObject,
                                                                    j_workdir: JString,
                                                                    j_network: jint,
                                                                    j_passphrase: JString,
                                                                    j_pd_passphrase: JString) -> jobject {
    let workdir = string_from_jstring(&env, j_workdir);
    let network = network_from_jint(j_network);

    let mut config_path = PathBuf::from(workdir);
    config_path.push(network.to_string());
    fs::create_dir_all(&config_path).expect(format!("unable to create config_path: {}", &config_path.to_str().unwrap()).as_str());

    let mut file_path = config_path.clone();
    file_path.push("btcdk.cfg");

    let passphrase = string_from_jstring(&env, j_passphrase);
    let pd_passphrase = env.get_string(j_pd_passphrase).ok();
    let pd_passphrase = pd_passphrase.iter()
        .map(|pd| pd.to_str().expect("error j_pd_passphrase JavaStr.to_str()"))
        .next();

    if let Ok(_config) = config::load(&file_path) {
        // do not init if a config already exists, return empty
        j_optional_empty(&env)
    } else {

        // create new wallet
        let (mnemonic_words, deposit_address, wallet) = Wallet::new(network, passphrase.as_str(), pd_passphrase);
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
        config::save(&config_path, &file_path, &config).expect("error saving config");

        // return config
        j_optional_init_result(&env, mnemonic_words.as_str(), deposit_address.as_str())
    }
}

// void org.btcdk.jni.BtcDkLib.start(String workDir, int network, boolean rescan)
// #[no_mangle]
// pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_start(env: JNIEnv, _: JObject, j_workdir: JString, j_network: jint, j_rescan: jboolean) {
//     let workdir = string_from_jstring(&env, j_workdir);
//     let network = network_from_jint(j_network);
//     let rescan = j_rescan == 1;
//
//     let mut config_path = PathBuf::from(workdir);
//     config_path.push(network.to_string());
//
//     let mut config_file_path = config_path.clone();
//     config_file_path.push("btcdk.cfg");
//
//     let config = config::load(&config_file_path).expect("can not open config file");
//
//     let mut chain_file_path = config_path.clone();
//     chain_file_path.push("btcdk.chain");
//
//     let mut chain_db = ChainDB::new(chain_file_path.as_path(), network).expect("can not open chain db");
//     chain_db.init().expect("can not initialize db");
//     let chain_db = Arc::new(RwLock::new(chain_db));
//
//     let db = open_db(&config_path);
//     let db = Arc::new(Mutex::new(db));
//
//     // get master account
//     let mut bitcoin_wallet;
//     let mut master_account = MasterAccount::from_encrypted(
//         hex::decode(config.encryptedwalletkey).expect("encryptedwalletkey is not hex").as_slice(),
//         ExtendedPubKey::from_str(config.keyroot.as_str()).expect("keyroot is malformed"),
//         config.birth,
//     );
//
//     // load wallet from master account
//     {
//         let mut db = db.lock().unwrap();
//         let mut tx = db.transaction();
//         let account = tx.read_account(0, 0, network, config.lookahead).expect("can not read account 0/0");
//         master_account.add_account(account);
//         let account = tx.read_account(0, 1, network, config.lookahead).expect("can not read account 0/1");
//         master_account.add_account(account);
//         let account = tx.read_account(1, 0, network, 0).expect("can not read account 1/0");
//         master_account.add_account(account);
//         let coins = tx.read_coins(&mut master_account).expect("can not read coins");
//         bitcoin_wallet = Wallet::from_storage(coins, master_account);
//     }
//
//     // rescan chain if requested
//     if rescan {
//         let chain_db = chain_db.read().unwrap();
//         let mut after = None;
//         for cached_header in chain_db.iter_trunk_rev(None) {
//             if (cached_header.stored.header.time as u64) < config.birth {
//                 after = Some(cached_header.bitcoin_hash());
//                 break;
//             }
//         }
//         if let Some(after) = after {
//             info!("Re-scanning after block {}", &after);
//             let mut db = db.lock().unwrap();
//             let mut tx = db.transaction();
//             tx.rescan(&after).expect("can not re-scan");
//             tx.commit();
//             bitcoin_wallet.rescan();
//         }
//     }
//
//     let trunk = Arc::new(ChainDBTrunk { chaindb: chain_db.clone() });
//     info!("Wallet balance: {} satoshis {} available", bitcoin_wallet.balance(), bitcoin_wallet.available_balance(trunk.len(), |h| trunk.get_height(h)));
//
//     let content_store =
//         Arc::new(RwLock::new(
//             ContentStore::new(db.clone(), 0,
//                               trunk,
//                               bitcoin_wallet)
//                 .expect("can not initialize content store")));
//
//     // if let Some(http) = http_rpc {
//     //     let address = http.clone();
//     //     let store = content_store.clone();
//     //     let apikey = config.apikey.clone();
//     //     thread::Builder::new().name("http".to_string()).spawn(
//     //         move || start_api(&address, store, apikey)).expect("can not start http api");
//     // }
//
//     let mut thread_pool = ThreadPoolBuilder::new().name_prefix("futures ").create().expect("can not start thread pool");
//     P2PBitcoin::new(config.network, config.bitcoin_connections, config.bitcoin_peers, config.bitcoin_discovery, chain_db.clone(), db.clone(),
//                     content_store.clone(), config.birth).start(&mut thread_pool);
//
//     thread_pool.run(future::pending::<()>());
// }

// private functions

fn string_from_jstring(env: &JNIEnv, j_string: JString) -> String {
    let java_str = env.get_string(j_string).expect("error get_string j_string");
    let str = java_str.to_str().expect("error java_str.to_str");
    String::from(str)
}

fn j_optional_empty(env: &JNIEnv) -> jobject {
    // Optional.empty())
    let j_result = env.call_static_method(
        "java/util/Optional",
        "empty",
        "()Ljava/util/Optional;",
        &[]).expect("error Optional.empty()")
        .l().expect("error converting Optional.empty() jvalue to jobject");

    j_result.into_inner()
}

fn network_from_jint(network_enum_ordinal: jint) -> Network {
    match network_enum_ordinal {
        0 => Some(Network::Bitcoin),
        1 => Some(Network::Testnet),
        2 => Some(Network::Regtest),
        _ => None
    }.expect("invalid network enum ordinal")
}

fn jint_from_network(network: Network) -> jint {
    match network {
        Network::Bitcoin => 0,
        Network::Testnet => 1,
        Network::Regtest => 2,
    }
}

// InitResult(String mnemonicWords, String depositAddress)
fn j_optional_init_result(env: &JNIEnv, mnemonic_words: &str, deposit_address: &str) -> jobject {
    let mnemonic_words = env.new_string(mnemonic_words)
        .expect("error new_string mnemonic_words");
    let deposit_address = env.new_string(deposit_address)
        .expect("error new_string deposit_address");

    // org.btcdk.jni.InitResult
    // Optional.of(InitResult(String mnemonicWords, String depositAddress))
    let j_result = env.new_object(
        "org/btcdk/jni/InitResult",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(mnemonic_words.into()), JValue::Object(deposit_address.into())],
    ).expect("error new_object InitResult");

    let j_result = env.call_static_method(
        "java/util/Optional",
        "of",
        "(Ljava/lang/Object;)Ljava/util/Optional;",
        &[JValue::Object(j_result)]).expect("error Optional.of(InitResult)")
        .l().expect("error converting Optional.of() jvalue to jobject");

    j_result.into_inner()
}

// Config(int networkEnumOrdinal, String[] bitcoinPeers, int bitcoinConnections, boolean bitcoinDiscovery)
fn j_optional_config(env: &JNIEnv, config: &Config) -> jobject {
    let j_network_enum_ordinal: JValue = jint_from_network(config.network).into();

    // return peer addresses as JString vector
    let j_bitcoin_peer_vec: Vec<JString> = config.bitcoin_peers.iter()
        .map(|s| s.to_string())
        .map(|a| env.new_string(a).expect("error env.new_string(a)"))
        .collect();

    let j_bitcoin_peer_arr: jobjectArray = env.new_object_array(i32::try_from(j_bitcoin_peer_vec.len()).unwrap(),
                                                                env.find_class("java/lang/String").expect("error env.find_class(String)"),
                                                                env.new_string("").expect("error env.new_string()").into())
        .expect("error env.new_object_array()");


    for i in 0..(j_bitcoin_peer_vec.len()) {
        env.set_object_array_element(j_bitcoin_peer_arr, i32::try_from(i).unwrap(),
                                     j_bitcoin_peer_vec[i].into()).expect("error set_object_array_element");
    }

    let j_bitcoin_connections: JValue = jint::try_from(config.bitcoin_connections)
        .expect("error converting bitcoin_connections to jint").into();

    let j_bitcoin_discover: JValue = jboolean::try_from(config.bitcoin_discovery)
        .expect("error converting bitcoin_discovery to jboolean").into();

    // org.btcdk.jni.Config
    // Optional.of(Config())
    let j_result = env.new_object(
        "org/btcdk/jni/Config",
        "(I[Ljava/lang/String;IZ)V",
        &[j_network_enum_ordinal, JValue::Object(j_bitcoin_peer_arr.into()),
            j_bitcoin_connections, j_bitcoin_discover],
    ).expect("error new_object Config");

    let j_result = env.call_static_method(
        "java/util/Optional",
        "of",
        "(Ljava/lang/Object;)Ljava/util/Optional;",
        &[JValue::Object(j_result)]).expect("error Optional.of(InitResult)")
        .l().expect("error converting Optional.of() jvalue to jobject");

    j_result.into_inner()
}

// fn open_db(config_path: &Path) -> DB {
//     let mut db_path = PathBuf::from(config_path);
//     db_path.push("btcdk.db");
//     let db = DB::new(db_path.as_path()).expect(format!("Can't open DB {}", db_path.to_str().expect("can't get db_path")).as_str());
//     db
// }

#[cfg(test)]
mod test {


}
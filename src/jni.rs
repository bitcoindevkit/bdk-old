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

use std::convert::TryFrom;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

use bitcoin::{Address, Network};
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use jni::sys::{jboolean, jint, jlong, jobject, jobjectArray};
use log::{error, info, warn};

use crate::api::{balance, BalanceAmt, deposit_addr, init_config, InitResult, load_config, remove_config, start, stop, update_config, withdraw, WithdrawTx};
use crate::config::Config;

// public API

// void org.bdk.jni.BdkLib.initLogger()

#[no_mangle]
#[cfg(feature = "android")]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_initLogger(_: JNIEnv, _: JObject) {
    android_log::init("BDK").unwrap();
    info!("android logger initialized");
}

#[no_mangle]
#[cfg(feature = "java")]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_initLogger(_: JNIEnv, _: JObject) {
    env_logger::init();
    info!("java logger initialized");
}

// Optional<Config> org.bdk.jni.BdkLib.loadConfig(String workDir, int network)
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_loadConfig(env: JNIEnv, _: JObject,
                                                            j_work_dir: JString,
                                                            j_network: jint) -> jobject {
    let work_dir = string_from_jstring(&env, j_work_dir);
    let work_dir = PathBuf::from(work_dir);
    let network = network_from_jint(j_network);

    match load_config(work_dir, network) {
        Ok(config) => j_optional_config(&env, &config),
        Err(_err) => j_optional_empty(&env)
    }
}

// Optional<Config> org.bdk.jni.BdkLib.removeConfig(String workDir, int network)
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_removeConfig(env: JNIEnv, _: JObject,
                                                              j_work_dir: JString,
                                                              j_network: jint) -> jobject {
    let work_dir = string_from_jstring(&env, j_work_dir);
    let work_dir = PathBuf::from(work_dir);
    let network = network_from_jint(j_network);

    match remove_config(work_dir, network) {
        Ok(config) => j_optional_config(&env, &config),
        Err(_err) => j_optional_empty(&env)
    }
}

// Optional<Config> org.bdk.jni.BdkLib.updateConfig(String workDir, int network, String[] bitcoinPeers, int bitcoinConnections, boolean bitcoinDiscovery)
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_updateConfig(env: JNIEnv, _: JObject,
                                                              j_work_dir: JString,
                                                              j_network: jint,
                                                              j_bitcoin_peers: jobjectArray,
                                                              j_bitcoin_connections: jint,
                                                              j_bitcoin_discovery: jboolean) -> jobject {
    let work_dir = string_from_jstring(&env, j_work_dir);
    let work_dir = PathBuf::from(work_dir);
    let network = network_from_jint(j_network);

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

        bitcoin_peers.push(bitcoin_peer_addr);
    }

    let bitcoin_connections = usize::try_from(j_bitcoin_connections).expect("usize::try_from(j_bitcoin_connections");
    let bitcoin_discovery = j_bitcoin_discovery == 1;

    match update_config(work_dir, network, bitcoin_peers, bitcoin_connections, bitcoin_discovery) {
        Ok(updated_config) => j_optional_config(&env, &updated_config),
        Err(_err) => j_optional_empty(&env)
    }
}

// Optional<InitResult> org.bdk.jni.BdkLib.initConfig(String workDir, int network, String passphrase, String pdPassphrase)
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_initConfig(env: JNIEnv, _: JObject,
                                                            j_work_dir: JString,
                                                            j_network: jint,
                                                            j_passphrase: JString,
                                                            j_pd_passphrase: JString) -> jobject {
    let work_dir = string_from_jstring(&env, j_work_dir);
    let work_dir = PathBuf::from(work_dir);
    let network = network_from_jint(j_network);

    let passphrase = string_from_jstring(&env, j_passphrase);
    let passphrase = passphrase.as_str();
    let pd_passphrase = env.get_string(j_pd_passphrase).ok();
    let pd_passphrase = pd_passphrase.iter()
        .map(|pd| pd.to_str().expect("error j_pd_passphrase JavaStr.to_str()"))
        .next();

    match init_config(work_dir, network, passphrase, pd_passphrase) {
        Ok(None) => {
            // do not init if a config already exists, return empty
            j_optional_empty(&env)
        }
        Ok(Some(init_result)) => {
            // return config
            j_optional_init_result(&env, init_result)
        }
        Err(_err) => {
            // TODO throw java exception
            j_optional_empty(&env)
        }
    }
}

// void org.bdk.jni.BdkLib.start(String workDir, int network, boolean rescan)
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_start(env: JNIEnv, _: JObject, j_work_dir: JString, j_network: jint, j_rescan: jboolean) {
    let work_dir = string_from_jstring(&env, j_work_dir);
    let work_dir = PathBuf::from(work_dir);
    let network = network_from_jint(j_network);
    let rescan = j_rescan == 1;

    match start(work_dir, network, rescan) {
        Ok(_) => (),
        Err(_e) => {
            // TODO throw java exception
            error!("Could not start wallet.");
            ()
        }
    }
}

// void org.bdk.jni.BdkLib.stop()
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_stop(_: JNIEnv, _: JObject) {
    stop()
}

// Option<BalanceAmt> org.bdk.jni.BdkLib.balance()
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_balance(env: JNIEnv, _: JObject) -> jobject {
    match balance() {
        Ok(balance_amt) => {
            // return wallet balance amt
            j_optional_balance_amt_result(&env, balance_amt)
        },
        Err(_e) => {
            // TODO throw java exception
            error!("Could not get wallet balance amt.");
            j_optional_empty(&env)
        }
    }
}

// new Address(String address, int network, Optional<String> type)
// Address org.bdk.jni.BdkLib.depositAddress()
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_depositAddress(env: JNIEnv, _: JObject) -> jobject {
    let address = deposit_addr();
    j_address(&env, &address)
}

// new WithdrawTx(String txid, long fee)
// WithdrawTx org.bdk.jni.BdkLib.withdraw(String passphrase, String address, long feePerVbyte, long amount)
#[no_mangle]
pub unsafe extern fn Java_org_bdk_jni_BdkLib_withdraw(env: JNIEnv, _: JObject,
                                                          j_passphrase: JString,
                                                          j_address: JString,
                                                          j_fee_per_vbyte: jlong,
                                                          j_amount: jlong) -> jobject {

    let passphrase = string_from_jstring(&env, j_passphrase);
    let address = string_from_jstring(&env, j_address);
    let address = Address::from_str(address.as_str()).unwrap();

    let fee_per_vbyte = u64::try_from(j_fee_per_vbyte).unwrap();
    let amount = u64::try_from(j_amount).unwrap();

    let withdraw_tx = withdraw(passphrase, address, fee_per_vbyte, Some(amount)).unwrap();
    j_withdraw_tx(&env, &withdraw_tx)
}


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

// InitResult(String mnemonicWords, Address depositAddress)
fn j_optional_init_result(env: &JNIEnv, init_result: InitResult) -> jobject {
    let mnemonic_words = env.new_string(init_result.mnemonic_words)
        .expect("error new_string mnemonic_words");
    let deposit_address: jobject = j_address(&env, &init_result.deposit_address);

    // org.bdk.jni.InitResult
    // Optional.of(InitResult(String mnemonicWords, String depositAddress))
    let j_result = env.new_object(
        "org/bdk/jni/InitResult",
        "(Ljava/lang/String;Lorg/bdk/jni/Address;)V",
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

// new BalanceAmt(long,long)
fn j_optional_balance_amt_result(env: &JNIEnv, balance_amt: BalanceAmt) -> jobject {
    let bal = JValue::Long(jlong::try_from(balance_amt.balance).unwrap());
    let conf = JValue::Long(jlong::try_from(balance_amt.confirmed).unwrap());
    let j_result = env.new_object(
        "org/bdk/jni/BalanceAmt",
        "(JJ)V",
        &[bal, conf],
    ).expect("error new_object BalanceAmt");

    let j_result = env.call_static_method(
        "java/util/Optional",
        "of",
        "(Ljava/lang/Object;)Ljava/util/Optional;",
        &[JValue::Object(j_result)]).expect("error Optional.of(BalanceAmt)")
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

    // org.bdk.jni.Config
    // Optional.of(Config())
    let j_result = env.new_object(
        "org/bdk/jni/Config",
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

fn j_optional_string(env: &JNIEnv, string: &String) -> jobject {
    let j_string = env.new_string(string).unwrap();

    // java.lang.String
    // Optional.of(String)
    let j_result = env.call_static_method(
        "java/util/Optional",
        "of",
        "(Ljava/lang/Object;)Ljava/util/Optional;",
        &[JValue::Object(j_string.into())]).expect("error Optional.of(String)")
        .l().expect("error converting Optional.of() jvalue to jobject");

    j_result.into_inner()
}

// org.bdk.jni.Address(String address, int networkEnumOrdinal, Optional<String> type)
fn j_address(env: &JNIEnv, address: &Address) -> jobject {
    let addr = address.to_string();
    let addr = env.new_string(addr).unwrap();
    let addr = JValue::Object(addr.into());
    let addr_network = jint_from_network(address.network);
    let addr_network = JValue::Int(addr_network);
    let addr_type = address.address_type().map(|t| t.to_string());
    let addr_type: jobject = match addr_type {
        Some(at) => j_optional_string(&env, &at),
        None => j_optional_empty(&env)
    };
    let addr_type = JValue::Object(addr_type.into());

    let j_result = env.new_object(
        "org/bdk/jni/Address",
        "(Ljava/lang/String;ILjava/util/Optional;)V",
        &[addr, addr_network, addr_type],
    ).expect("error new_object Address");

    j_result.into_inner()
}

// org.bdk.jni.WithdrawTx(String txid, long fee)
fn j_withdraw_tx(env: &JNIEnv, withdraw_tx: &WithdrawTx) -> jobject {
    let txid = withdraw_tx.txid.to_string();
    let txid = env.new_string(txid).unwrap();
    let fee = i64::try_from(withdraw_tx.fee).unwrap();

    let j_result = env.new_object(
        "org/bdk/jni/WithdrawTx",
        "(Ljava/lang/String;J)V",
        &[JValue::Object(txid.into()), JValue::Long(fee)],
    ).expect("error new_object WithdrawTx");

    j_result.into_inner()
}

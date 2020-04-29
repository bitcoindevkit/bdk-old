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

use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use jni::sys::{jboolean, jint, jobject, jobjectArray};

use crate::api::{init_config, InitResult, load_config, remove_config, start, update_config};
use crate::config::Config;
use bitcoin::Network;

// Optional<Config> org.btcdk.jni.BtcDkLib.loadConfig(String workDir, int network)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_loadConfig(env: JNIEnv, _: JObject,
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

// Optional<Config> org.btcdk.jni.BtcDkLib.removeConfig(String workDir, int network)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_removeConfig(env: JNIEnv, _: JObject,
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

// Optional<Config> org.btcdk.jni.BtcDkLib.updateConfig(String workDir, int network, String[] bitcoinPeers, int bitcoinConnections, boolean bitcoinDiscovery)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_updateConfig(env: JNIEnv, _: JObject,
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

        let index = usize::try_from(i).expect("usize::try_from(bitcoin_peers_length");
        bitcoin_peers.push(bitcoin_peer_addr);
    }

    let bitcoin_connections = usize::try_from(j_bitcoin_connections).expect("usize::try_from(j_bitcoin_connections");
    let bitcoin_discovery = j_bitcoin_discovery == 1;

    match update_config(work_dir, network, bitcoin_peers, bitcoin_connections, bitcoin_discovery) {
        Ok(updated_config) => j_optional_config(&env, &updated_config),
        Err(_err) => j_optional_empty(&env)
    }
}

// Optional<InitResult> org.btcdk.jni.BtcDkLib.initConfig(String workDir, int network, String passphrase, String pdPassphrase)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_initConfig(env: JNIEnv, _: JObject,
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

// void org.btcdk.jni.BtcDkLib.start(String workDir, int network, boolean rescan)
#[no_mangle]
pub unsafe extern fn Java_org_btcdk_jni_BtcDkLib_start(env: JNIEnv, _: JObject, j_work_dir: JString, j_network: jint, j_rescan: jboolean) {
    let work_dir = string_from_jstring(&env, j_work_dir);
    let work_dir = PathBuf::from(work_dir);
    let network = network_from_jint(j_network);
    let rescan = j_rescan == 1;

    match start(work_dir, network, rescan) {
        Ok(_) => (),
        // TODO throw java exception
        Err(_e) => ()
    }
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

// InitResult(String mnemonicWords, String depositAddress)
fn j_optional_init_result(env: &JNIEnv, init_result: InitResult) -> jobject {
    let mnemonic_words = env.new_string(init_result.mnemonic_words)
        .expect("error new_string mnemonic_words");
    let deposit_address = env.new_string(init_result.deposit_address)
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

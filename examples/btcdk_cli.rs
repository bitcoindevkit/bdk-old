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
extern crate btcdk;

use env_logger::Env;
use std::path::PathBuf;
use bitcoin::{Network, Address};
use std::net::SocketAddr;
use std::str::FromStr;
use log::{info, warn, error};
use btcdk::api::{init_config, update_config, start, balance, deposit_addr, withdraw};
use std::thread;
use btcdk::api;
use bitcoin_hashes::core::time::Duration;

const PASSPHRASE: &str = "correct horse battery staple";
const PD_PASSPHRASE_1: &str = "test123";

fn main() {
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    info!("main()");

    let work_dir: PathBuf = PathBuf::from(".");

    let inited = init_config(work_dir.clone(), Network::Regtest,
                             PASSPHRASE, Some(PD_PASSPHRASE_1)).unwrap();
    let peer1 = SocketAddr::from_str("127.0.0.1:9333").unwrap();
    let peer2 = SocketAddr::from_str("127.0.0.1:19333").unwrap();

    let updated = update_config(work_dir.clone(), Network::Regtest,
                                vec!(peer1, peer2),
                                2, false).unwrap();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1000));
        let balanceAmt = balance();
        info!("balance: {:?}", balanceAmt);

        let deposit_addr = deposit_addr();
        info!("deposit addr: {:?}", deposit_addr);

        let withdrawTx = withdraw(PASSPHRASE.to_string(),
                                Address::from_str("bcrt1q9dugqfjn3p3rrcvdw68zh790pd8g4vm3hmam09").unwrap(),
                                10000, None);
        match withdrawTx {
            Ok(wtx) => info!("withdraw tx: {:?}", wtx),
            Err(e) => error!("withdraw error: {:?}", e),
        }

        let balanceAmt = balance();
        info!("balance: {:?}", balanceAmt);

        let withdrawTx = withdraw(PASSPHRASE.to_string(),
                                Address::from_str("bcrt1q9dugqfjn3p3rrcvdw68zh790pd8g4vm3hmam09").unwrap(),
                                1, Some(1000000));
        match withdrawTx {
            Ok(wtx) => info!("withdraw tx: {:?}", wtx),
            Err(e) => error!("withdraw error: {:?}", e),
        }
    });

    info!("Before start.");

    start(work_dir.clone(), Network::Regtest, false);
}
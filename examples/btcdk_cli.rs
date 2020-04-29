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
use log::{info, warn};
use btcdk::api::{init_config, update_config, start, send_cmd, Event, Command};
use std::thread;
use btcdk::api;
use btcdk::api::Command::GetBalance;

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

    // user output thread
    thread::spawn(move || {
        loop {
            match api::recv_evt() {
                Ok(evt) => {
                    info!("received evt: {:?}", evt);
                    match evt {
                        Event::Stopped => {
                            warn!("stopped.");
                            break;
                        },
                        Event::Balance { balance, confirmed } => {
                            info!("balance: {}, confirmed: {}", balance, confirmed);
                        },
                        Event::DepositAddress { address } => {
                            info!("deposit address: {}", address);
                        },
                        Event::WithdrawTx { txid } => {
                            info!("withdraw txid: {}", txid);
                        }
                        _ => { // do nothing }
                        }
                    }
                }
                Err(err) => {
                    warn!("receive error: {}", err);
                }
            }
        }
    });

    info!("Before start.");

    send_cmd(Command::GetBalance);
    send_cmd(Command::GetDepositAddress);
    send_cmd(Command::Withdraw {
        passphrase: PASSPHRASE.to_string(),
        target_address: Address::from_str("bcrt1q9dugqfjn3p3rrcvdw68zh790pd8g4vm3hmam09").unwrap(),
        fee_per_byte: 10000,
        amount: Some(1000000)
    });
    //send_cmd(Command::Stop);

    start(work_dir.clone(), Network::Regtest, false);
}
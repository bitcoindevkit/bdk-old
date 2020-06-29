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
extern crate bdk;

#[macro_use]
extern crate clap;

mod app;

use env_logger::Env;
use std::path::PathBuf;
use bitcoin::{Network, Address};
use std::net::{SocketAddr, AddrParseError};
use std::str::FromStr;
use log::{debug, info, warn, error};
use bdk::api::{init_config, update_config, start, balance, deposit_addr, withdraw, stop};
use std::thread;
use bdk::api;
use bitcoin_hashes::core::time::Duration;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use clap::App;
use bdk::config::Config;
use bdk::error::Error;
use futures::StreamExt;
use std::convert::TryFrom;
use std::cmp::max;

const PASSPHRASE: &str = "correct horse battery staple";
const PD_PASSPHRASE_1: &str = "test123";

fn main() -> Result<(), Error> {
    let cli = app::cli().get_matches();
    let logging = cli.value_of("logging").unwrap_or("info");
    env_logger::from_env(Env::default().default_filter_or(logging)).init();

    let connections = cli.value_of("connections").map(|c| c.parse::<usize>().unwrap()).unwrap_or(5);
    let directory = cli.value_of("directory").unwrap_or(".");
    let discovery = cli.value_of("discovery").map(|d| d == "on").unwrap_or(true);
    let network = cli.value_of("network").unwrap_or("testnet");
    let password = cli.value_of("password").expect("password is required");
    let peers = cli.values_of("peers").map(|a| a.collect::<Vec<&str>>()).unwrap_or(Vec::new());

    let work_dir: PathBuf = PathBuf::from(directory);
    let mut history_file = work_dir.clone();
    history_file.push(network);
    history_file.push("history.txt");
    let history_file = history_file.as_path();
    info!("history file: {:?}", history_file);

    let network = network.parse::<Network>().unwrap();

    info!("logging level: {}", logging);
    info!("working directory: {:?}", work_dir);
    info!("discovery: {:?}", discovery);
    info!("network: {}", network);
    info!("peers: {:?}", peers);

    let init_result = api::init_config(work_dir.clone(), network, password, None);

    match init_result {
        Ok(Some(init_result)) => {
            warn!("created new wallet, seed words: {}", init_result.mnemonic_words);
            info!("first deposit address: {}", init_result.deposit_address);
        }
        Ok(None) => {
            info!("wallet exists");
        }
        Err(e) => {
            error!("config error: {:?}", e);
        }
    };

    let peers = peers.into_iter()
        .map(|p| SocketAddr::from_str(p))
        .collect::<Result<Vec<SocketAddr>, AddrParseError>>()?;

    let connections = max(peers.len(), connections);

    info!("peer connections: {}", connections);

    let config = api::update_config(work_dir.clone(), network, peers, connections, discovery).unwrap();
    debug!("config: {:?}", config);

    let mut rl = Editor::<()>::new();

    if rl.load_history(history_file).is_err() {
        info!("No previous history.");
    }

    let p2p_thread = thread::spawn(move || {
        info!("starting p2p thread");
        api::start(work_dir.clone(), network, false);
    });

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                info!("Line: {}", line);
            }
            Err(ReadlineError::Interrupted) => {
                info!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                info!("CTRL-D");
                break;
            }
            Err(err) => {
                error!("Error: {:?}", err);
                break;
            }
        }
    }
    api::stop();
    rl.save_history(history_file).unwrap();
    p2p_thread.join().unwrap();
    Ok(())

    // let inited = init_config(work_dir.clone(), Network::Testnet,
    //                          PASSPHRASE, Some(PD_PASSPHRASE_1)).unwrap();
    // let peer1 = SocketAddr::from_str("127.0.0.1:9333").unwrap();
    // let peer2 = SocketAddr::from_str("127.0.0.1:19333").unwrap();
    //
    // let updated = update_config(work_dir.clone(), Network::Testnet,
    //                             vec![peer1, peer2],
    //                             2, false).unwrap();

    // thread::spawn(move || {
    //     thread::sleep(Duration::from_millis(1000));
    //     let balanceAmt = balance();
    //     info!("balance: {:?}", balanceAmt);
    //
    //     let deposit_addr = deposit_addr();
    //     info!("deposit addr: {:?}, type: {:?}", deposit_addr, deposit_addr.address_type());
    //
    //     // let withdrawTx = withdraw(PASSPHRASE.to_string(),
    //     //                         Address::from_str("bcrt1q9dugqfjn3p3rrcvdw68zh790pd8g4vm3hmam09").unwrap(),
    //     //                         10000, None);
    //     // match withdrawTx {
    //     //     Ok(wtx) => info!("withdraw tx: {:?}", wtx),
    //     //     Err(e) => error!("withdraw error: {:?}", e),
    //     // }
    //     //
    //     // let balanceAmt = balance();
    //     // info!("balance: {:?}", balanceAmt);
    //     //
    //     // thread::sleep(Duration::from_secs(30));
    //     //
    //     // let withdrawTx = withdraw(PASSPHRASE.to_string(),
    //     //                         Address::from_str("bcrt1q9dugqfjn3p3rrcvdw68zh790pd8g4vm3hmam09").unwrap(),
    //     //                         1, Some(1000000));
    //     // match withdrawTx {
    //     //     Ok(wtx) => info!("withdraw tx: {:?}", wtx),
    //     //     Err(e) => error!("withdraw error: {:?}", e),
    //     // }
    //
    //     thread::sleep(Duration::from_secs(240));
    //
    //     let balanceAmt = balance();
    //     info!("balance: {:?}", balanceAmt);
    //
    //     stop();
    // });
    //
    // info!("Before start.");
    //
    // start(work_dir.clone(), Network::Testnet, false);
}
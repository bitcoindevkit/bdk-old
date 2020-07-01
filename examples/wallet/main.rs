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

use std::cmp::max;
use std::convert::TryFrom;
use std::net::{AddrParseError, SocketAddr};
use std::path::{PathBuf, Path};
use std::str::FromStr;
use std::thread;

use bitcoin::{Address, Network};
use bitcoin_hashes::core::time::Duration;
use clap::App;
use futures::StreamExt;
use log::{debug, error, info, warn, LevelFilter};
use rustyline::Editor;
use rustyline::error::ReadlineError;

use bdk::api::{balance, deposit_addr, init_config, start, stop, update_config, withdraw};
use bdk::api;
use bdk::config::Config;
use bdk::error::Error;
use std::process::ChildStderr;
use chrono::Local;

mod ui;

const PASSPHRASE: &str = "correct horse battery staple";
const PD_PASSPHRASE_1: &str = "test123";

fn main() -> Result<(), Error> {
    let cli = ui::cli().get_matches();
    let log_level = cli.value_of("logging").unwrap_or("info");

    let connections = cli.value_of("connections").map(|c| c.parse::<usize>().unwrap()).unwrap_or(5);
    let directory = cli.value_of("directory").unwrap_or(".");
    let discovery = cli.value_of("discovery").map(|d| d == "on").unwrap_or(true);
    let network = cli.value_of("network").unwrap_or("testnet");
    let password = cli.value_of("password").expect("password is required");
    let peers = cli.values_of("peers").map(|a| a.collect::<Vec<&str>>()).unwrap_or(Vec::new());

    let work_dir: PathBuf = PathBuf::from(directory);
    let mut log_file = work_dir.clone();
    log_file.push(network);
    log_file.push("wallet.log");
    let log_file = log_file.as_path();
    let log_level = LevelFilter::from_str(log_level).unwrap();

    setup_logger(log_file, log_level);

    let mut history_file = work_dir.clone();
    history_file.push(network);
    history_file.push("history.txt");
    let history_file = history_file.as_path();
    info!("history file: {:?}", history_file);

    let network = network.parse::<Network>().unwrap();

    println!("logging level: {}", log_level);
    println!("working directory: {:?}", work_dir);
    println!("discovery: {:?}", discovery);
    println!("network: {}", network);
    println!("peers: {:?}", peers);

    let init_result = api::init_config(work_dir.clone(), network, password, None);

    match init_result {
        Ok(Some(init_result)) => {
            println!("created new wallet, seed words: {}", init_result.mnemonic_words);
            println!("first deposit address: {}", init_result.deposit_address);
        }
        Ok(None) => {
            println!("wallet exists");
        }
        Err(e) => {
            println!("config error: {:?}", e);
        }
    };

    let peers = peers.into_iter()
        .map(|p| SocketAddr::from_str(p))
        .collect::<Result<Vec<SocketAddr>, AddrParseError>>()?;

    let connections = max(peers.len(), connections);

    println!("peer connections: {}", connections);

    let config = api::update_config(work_dir.clone(), network, peers, connections, discovery).unwrap();
    debug!("config: {:?}", config);

    let mut rl = Editor::<()>::new();

    if rl.load_history(history_file).is_err() {
        println!("No previous history.");
    }

    let p2p_thread = thread::spawn(move || {
        println!("starting p2p thread");
        api::start(work_dir.clone(), network, false);
    });

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let split_line = line.split(' ');
                let repl_matches = ui::repl().get_matches_from_safe(split_line);
                if repl_matches.is_ok() {
                    if let (c, Some(a)) = repl_matches.unwrap().subcommand() {
                        debug!("command: {}, args: {:?}", c, a);
                        rl.add_history_entry(line.as_str());
                        match c {
                            "stop" => {
                                break;
                            }
                            "balance" => {
                                let balance_amt = api::balance().unwrap();
                                println!("balance: {}, confirmed: {}", balance_amt.balance, balance_amt.confirmed);
                            }
                            "deposit" => {
                               let deposit_addr = api::deposit_addr();
                                println!("deposit address: {}", deposit_addr);
                            }
                            "withdraw" => {
                                // passphrase: String, address: Address, fee_per_vbyte: u64, amount: Option<u64>
                                let password = a.value_of("password").unwrap().to_string();
                                let address = Address::from_str(a.value_of("address").unwrap()).unwrap();
                                let fee = a.value_of("fee").unwrap().parse::<u64>().unwrap();
                                let amount = Some(a.value_of("amount").unwrap().parse::<u64>().unwrap());
                                let withdraw_tx = api::withdraw(password, address, fee, amount).unwrap();
                                println!("withdraw tx id: {}, fee: {}", withdraw_tx.txid, withdraw_tx.fee);
                            }
                            _ => {
                                println!("command '{}' is not implemented", c);
                            }
                        }
                    }
                } else {
                    let err = repl_matches.err().unwrap();
                    println!("{}", err);
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history(history_file).unwrap();
    println!("stopping");
    api::stop();
    p2p_thread.join().unwrap();
    println!("stopped");
    Ok(())
}

fn setup_logger(file: &Path, level: LevelFilter) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(fern::log_file(file)?)
        .apply()?;
    Ok(())
}
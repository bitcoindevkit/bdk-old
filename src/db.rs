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

use std::collections::HashSet;
use std::hash::Hasher;
use std::io;
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use bitcoin::{Network, OutPoint, PublicKey, Script, TxOut};
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::util::bip32::ExtendedPubKey;
use bitcoin_hashes::{sha256, sha256d};
use bitcoin_hashes::hex::FromHex;
use bitcoin_wallet::account::{Account, AccountAddressType, KeyDerivation, MasterAccount};
use bitcoin_wallet::coins::{Coin, Coins};
use bitcoin_wallet::proved::ProvedTransaction;
use byteorder::{ByteOrder, LittleEndian};
use rand::{Rng, RngCore, thread_rng};
use rand_distr::Poisson;
use rusqlite::{Connection, NO_PARAMS, OptionalExtension, ToSql, Transaction};
use rusqlite::types::{Null, ValueRef};
use siphasher::sip::SipHasher;

use crate::error::Error;

pub type SharedDB = Arc<Mutex<DB>>;

const ADDRESS_SLOTS: u64 = 10000;

pub struct DB {
    connection: Connection
}

impl DB {
    pub fn memory() -> Result<DB, Error> {
        Ok(DB { connection: Connection::open_in_memory()? })
    }

    pub fn new(path: &std::path::Path) -> Result<DB, Error> {
        Ok(DB { connection: Connection::open(path)? })
    }

    pub fn transaction(&mut self) -> TX {
        TX { tx: self.connection.transaction().expect("can not start db transaction") }
    }
}

pub struct TX<'db> {
    tx: Transaction<'db>
}

impl<'db> TX<'db> {
    pub fn commit(self) {
        self.tx.commit().expect("failed to commit db transaction");
    }

    pub fn rollback(self) {
        self.tx.rollback().expect("failed to roll back db transaction");
    }

    pub fn create_tables(&mut self) {
        self.tx.execute_batch(r#"
            create table if not exists seed (
                k0 number,
                k1 number
            );

            create table if not exists address (
                network text,
                slot number,
                ip text,
                connected number,
                last_seen number,
                banned number,
                primary key(network, slot)
            ) without rowid;

            create table if not exists account (
                account number,
                sub number,
                address_type number,
                master text,
                instantiated blob,
                primary key(account, sub)
            ) without rowid;

            create table if not exists coins (
                txid text,
                vout number,
                value number,
                script blob,
                account number,
                sub number,
                kix number,
                tweak text,
                csv number,
                proof blob,
                primary key(txid, vout)
            ) without rowid;

            create table if not exists processed (
                block text
            );

            create table if not exists txout (
                txid text primary key,
                tx blob,
                confirmed text,
                publisher blob,
                id text,
                term number
            ) without rowid;
        "#).expect("failed to create db tables");
    }

    pub fn rescan(&mut self, after: &sha256d::Hash) -> Result<(), Error> {
        self.tx.execute(r#"
            update processed set block = ?1
        "#, &[&after.to_string() as &dyn ToSql])?;
        self.tx.execute(r#"
            delete from txout
        "#, NO_PARAMS)?;
        self.tx.execute(r#"
            delete from coins
        "#, NO_PARAMS)?;
        Ok(())
    }

    pub fn store_txout(&mut self, tx: &bitcoin::Transaction, funding: Option<(&PublicKey, &sha256::Hash, u16)>) -> Result<(), Error> {
        if let Some((publisher, id, term)) = funding {
            self.tx.execute(r#"
            insert or replace into txout (txid, tx, publisher, id, term) values (?1, ?2, ?3, ?4, ?5)
        "#, &[&tx.txid().to_string() as &dyn ToSql,
                &serialize(tx),
                &publisher.to_bytes(), &id.to_string(), &term])?;
        } else {
            self.tx.execute(r#"
            insert or replace into txout (txid, tx) values (?1, ?2)
        "#, &[&tx.txid().to_string() as &dyn ToSql,
                &serialize(tx)])?;
        }
        Ok(())
    }

    pub fn read_unconfirmed(&self) -> Result<Vec<(bitcoin::Transaction, Option<(PublicKey, sha256::Hash, u16)>)>, Error> {
        let mut result = Vec::new();
        // remove unconfirmed spend
        let mut query = self.tx.prepare(r#"
            select tx, publisher, id, term from txout where confirmed is null
        "#)?;
        for r in query.query_map(NO_PARAMS, |r| {
            Ok((r.get_unwrap::<usize, Vec<u8>>(0),
                match r.get_raw(1) {
                    ValueRef::Null => None,
                    ValueRef::Blob(publisher) => Some(publisher.to_vec()),
                    _ => panic!("unexpected tweak type")
                },
                match r.get_raw(2) {
                    ValueRef::Null => None,
                    ValueRef::Text(id) => Some(id.to_vec()),
                    _ => panic!("unexpected tweak type")
                },
                match r.get_raw(3) {
                    ValueRef::Null => None,
                    ValueRef::Integer(n) => Some(n as u16),
                    _ => panic!("unexpected tweak type")
                }))
        })? {
            let (tx, publisher, id, term) = r?;
            result.push(
                (deserialize::<bitcoin::Transaction>(tx.as_slice()).expect("can not deserialize stored transaction"),
                 if let Some(publisher) = publisher {
                     Some((PublicKey::from_slice(publisher.as_slice()).expect("stored publisher in txout not a pubkey"),
                           sha256::Hash::from_hex(std::str::from_utf8(id.unwrap().as_slice()).unwrap()).expect("stored id in txout not hex"),
                           term.unwrap()))
                 } else { None },
                ));
        }
        Ok(result)
    }

    pub fn read_seed(&mut self) -> Result<(u64, u64), Error> {
        if let Some(seed) = self.tx.query_row(r#"
            select k0, k1 from seed where rowid = 1
        "#, NO_PARAMS, |r| Ok(
            (r.get_unwrap::<usize, i64>(0) as u64,
             r.get_unwrap::<usize, i64>(1) as u64))).optional()? {
            return Ok(seed);
        } else {
            let k0 = thread_rng().next_u64();
            let k1 = thread_rng().next_u64();
            self.tx.execute(r#"
                insert or replace into seed (rowid, k0, k1) values (1, ?1, ?2)
            "#, &[&(k0 as i64) as &dyn ToSql, &(k1 as i64)])?;
            return Ok((k0, k1));
        }
    }

    pub fn read_processed(&mut self) -> Result<Option<sha256d::Hash>, Error> {
        Ok(self.tx.query_row(r#"
            select block from processed where rowid = 1
        "#, NO_PARAMS, |r| Ok(sha256d::Hash::from_hex(r.get_unwrap::<usize, String>(0).as_str())
            .expect("stored block not hex"))).optional()?)
    }

    pub fn store_processed(&mut self, block_id: &sha256d::Hash) -> Result<(), Error> {
        self.tx.execute(r#"
            insert or replace into processed (rowid, block) values (1, ?1)
        "#, &[&block_id.to_string() as &dyn ToSql])?;
        Ok(())
    }

    pub fn store_coins(&mut self, coins: &Coins) -> Result<(), Error> {
        self.tx.execute(r#"
            delete from coins;
        "#, NO_PARAMS)?;
        let mut statement = self.tx.prepare(r#"
            insert into coins (txid, vout, value, script, account, sub, kix, tweak, csv, proof)
            values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#)?;
        let proofs = coins.proofs();
        for (outpoint, coin) in coins.confirmed() {
            let proof = proofs.get(&outpoint.txid).expect("inconsistent wallet, missing proof");
            let tweak = if let Some(ref tweak) = coin.derivation.tweak { hex::encode(tweak) } else { "".to_string() };

            statement.execute(&[
                &outpoint.txid.to_string() as &dyn ToSql, &outpoint.vout,
                &(coin.output.value as i64), &coin.output.script_pubkey.to_bytes(),
                &coin.derivation.account, &coin.derivation.sub, &coin.derivation.kix,
                if tweak == "".to_string() {
                    &Null
                } else {
                    &tweak as &dyn ToSql
                },
                if let Some(ref csv) = coin.derivation.csv {
                    csv as &dyn ToSql
                } else {
                    &Null
                },
                &serde_cbor::ser::to_vec(&proof).expect("can not serialize proof")
            ])?;
        }

        for (unconfirmed, _) in self.read_unconfirmed()? {
            if let Some(proof) = proofs.values().find(|p| p.get_transaction().txid() == unconfirmed.txid()) {
                self.tx.execute(r#"
                    update txout set confirmed = ?1 where txid = ?2
                "#, &[&proof.get_block_hash().to_string() as &dyn ToSql, &proof.get_transaction().txid().to_string()])?;
            }
        }

        Ok(())
    }

    pub fn read_coins(&mut self, master_account: &mut MasterAccount) -> Result<Coins, Error> {
        // read confirmed
        let mut query = self.tx.prepare(r#"
            select txid, vout, value, script, account, sub, kix, tweak, csv, proof from coins
        "#)?;
        let mut coins = Coins::new();
        for r in query.query_map::<(OutPoint, Coin, ProvedTransaction), &[&dyn ToSql], _>(NO_PARAMS, |r| {
            Ok((
                OutPoint {
                    txid: sha256d::Hash::from_hex(r.get_unwrap::<usize, String>(0).as_str()).expect("transaction id not hex"),
                    vout: r.get_unwrap::<usize, u32>(1),
                },
                Coin {
                    output: TxOut {
                        script_pubkey: Script::from(r.get_unwrap::<usize, Vec<u8>>(3)),
                        value: r.get_unwrap::<usize, i64>(2) as u64,
                    },
                    derivation: KeyDerivation {
                        account: r.get_unwrap::<usize, u32>(4),
                        sub: r.get_unwrap::<usize, u32>(5),
                        kix: r.get_unwrap::<usize, u32>(6),
                        tweak: match r.get_raw(7) {
                            ValueRef::Null => None,
                            ValueRef::Text(tweak) => Some(hex::decode(tweak).expect("tweak not hex")),
                            _ => panic!("unexpected tweak type")
                        },
                        csv: match r.get_raw(8) {
                            ValueRef::Null => None,
                            ValueRef::Integer(i) => Some(i as u16),
                            _ => panic!("unexpected csv type")
                        },
                    },
                },
                serde_cbor::from_slice(r.get_unwrap::<usize, Vec<u8>>(9).as_slice()).expect("can not deserialize stored proof")
            ))
        })? {
            let (point, coin, proof) = r?;
            coins.add_confirmed(point, coin, proof);
        }

        // remove unconfirmed spend
        let mut query = self.tx.prepare(r#"
            select tx from txout where confirmed is null
        "#)?;
        for r in query.query_map(NO_PARAMS, |r| {
            Ok(r.get_unwrap::<usize, Vec<u8>>(0))
        })? {
            let tx = deserialize::<bitcoin::Transaction>(r?.as_slice()).expect("can not deserialize stored transaction");
            coins.process_unconfirmed_transaction(master_account, &tx);
        }
        Ok(coins)
    }

    pub fn store_master(&mut self, master: &MasterAccount) -> Result<usize, Error> {
        debug!("store master account");
        self.tx.execute(r#"
            delete from account;
        "#, NO_PARAMS)?;
        let mut inserted = 0;
        for (_, account) in master.accounts().iter() {
            inserted += self.store_account(account)?;
        }
        Ok(inserted)
    }

    pub fn store_account(&mut self, account: &Account) -> Result<usize, Error> {
        debug!("store account {}/{}", account.account_number(), account.sub_account_number());
        Ok(self.tx.execute(r#"
            insert or replace into account (account, address_type, sub, master, instantiated)
            values (?1, ?2, ?3, ?4, ?5)
        "#, &[&account.account_number() as &dyn ToSql,
            &account.address_type().as_u32(), &account.sub_account_number(), &account.master_public().to_string(),
            &serde_cbor::ser::to_vec(&account.instantiated())?],
        )?)
    }

    pub fn read_account(&mut self, account_number: u32, sub: u32, network: Network, look_ahead: u32) -> Result<Account, Error> {
        debug!("read account {}/{}", account_number, sub);
        Ok(self.tx.query_row(r#"
            select address_type, master, instantiated from account where account = ?1 and sub = ?2
        "#, &[&account_number as &dyn ToSql, &sub], |r| {
            Ok(Account::new_from_storage(
                AccountAddressType::from_u32(r.get_unwrap::<usize, u32>(0)),
                account_number,
                sub,
                ExtendedPubKey::from_str(r.get_unwrap::<usize, String>(1).as_str()).expect("malformed master public stored"),
                serde_cbor::from_slice(r.get_unwrap::<usize, Vec<u8>>(2).as_slice()).expect("malformed instantiated keys stored"),
                0,
                look_ahead,
                network,
            ))
        })?)
    }

    pub fn store_address(&mut self, network: &str, address: &SocketAddr, mut connected: u64, mut last_seen: u64, mut banned: u64) -> Result<usize, Error> {
        let (k0, k1) = self.read_seed()?;
        let mut siphasher = SipHasher::new_with_keys(k0, k1);
        siphasher.write(network.as_bytes());
        for a in NetAddress::new(address).address.iter() {
            let mut buf = [0u8; 2];
            LittleEndian::write_u16(&mut buf, *a);
            siphasher.write(&buf);
        }
        let slot = (siphasher.finish() % ADDRESS_SLOTS) as u16;
        if let Ok((oldip, oldconnect, oldls, oldban)) = self.tx.query_row(r#"
                select ip, connected, last_seen, banned from address where network = ?1 and slot = ?2
            "#, &[&network.to_string() as &dyn ToSql, &slot],
                                                                          |r| Ok(
                                                                              (SocketAddr::from_str(r.get_unwrap::<usize, String>(0).as_str()).expect("address stored in db should be parsable"),
                                                                               r.get_unwrap::<usize, i64>(1) as u64,
                                                                               r.get_unwrap::<usize, i64>(2) as u64,
                                                                               r.get_unwrap::<usize, i64>(3) as u64))) {
            // do not reduce last_seen or banned fields
            if oldip != *address {
                let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                const OLD_CONNECTION: u64 = 5 * 24 * 60 * 60;
                if oldban > 0 || oldconnect < now - OLD_CONNECTION {
                    return Ok(
                        self.tx.execute(r#"
                            insert or replace into address (network, slot, ip, connected, last_seen, banned) values (?1, ?2, ?3, ?4, ?5, ?6)
                        "#, &[&network.to_string() as &dyn ToSql, &slot, &address.to_string(),
                            &(connected as i64), &(last_seen as i64), &(banned as i64)])?
                    );
                }
            } else {
                connected = std::cmp::max(oldconnect as u64, connected);
                last_seen = std::cmp::max(oldls as u64, last_seen);
                banned = std::cmp::max(oldban as u64, banned);
                return Ok(self.tx.execute(r#"
                        insert or replace into address (network, slot, ip, connected, last_seen, banned) values (?1, ?2, ?3, ?4, ?5, ?6)
                    "#, &[&network.to_string() as &dyn ToSql, &slot, &address.to_string(),
                    &(connected as i64), &(last_seen as i64), &(banned as i64)])?);
            }
            Ok(0)
        } else {
            Ok(
                self.tx.execute(r#"
                insert or replace into address (network, slot, ip, connected, last_seen, banned) values (?1, ?2, ?3, ?4, ?5, ?6)
            "#, &[&network.to_string() as &dyn ToSql, &slot, &address.to_string(),
                    &(connected as i64), &(last_seen as i64), &(banned as i64)])?
            )
        }
    }

    // get an address not banned during the last day
    // the probability to be selected is exponentially higher for those with higher last_seen time
    // TODO mark tried connections, build slots instead of storing all. Replace only if not tried for long or banned
    pub fn get_an_address(&self, network: &str, other_than: Arc<Mutex<HashSet<SocketAddr>>>) -> Result<Option<SocketAddr>, Error> {
        const BAN_TIME: u64 = 60 * 60 * 24; // a day

        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let mut statement = self.tx.prepare(r#"
            select ip from address where network = ?2 and banned < ?1 order by last_seen desc
        "#)?;
        let other_than = other_than.lock().unwrap();
        let eligible = statement.query_map::<SocketAddr, _, _>(
            &[&((now - BAN_TIME) as i64) as &dyn ToSql, &network.to_string()],
            |row| {
                let s = row.get_unwrap::<usize, String>(0);
                let addr = SocketAddr::from_str(s.as_str()).expect("address stored in db should be parsable");
                Ok(addr)
            })?
            .filter_map(|socket|
                match socket {
                    Ok(a) => if !other_than.contains(&a) { Some(a) } else { None },
                    Err(_) => None
                }).collect::<Vec<_>>();
        let len = eligible.len();
        if len == 0 {
            return Ok(None);
        }
        Ok(Some(
            eligible[
                std::cmp::min(len - 1, thread_rng().sample::<f64, _>(
                    Poisson::new(len as f64 / 4.0).unwrap()) as usize)]))
    }
}


pub fn init(config_path: &Path, coins: &Coins, master: &MasterAccount) {
    let mut db = new(&config_path);
    {
        let mut tx = db.transaction();
        tx.create_tables();
        tx.commit();
    }
    {
        let mut tx = db.transaction();
        tx.store_coins(coins).expect("can not store new wallet's coins");
        tx.store_master(master).expect("can not store new master account");
        tx.commit();
    }
}

pub fn new(config_path: &Path) -> DB {
    let mut db_path = PathBuf::from(config_path);
    db_path.push("bdk.db");
    DB::new(db_path.as_path()).expect("can not open database")
}

#[derive(Clone, Copy, Serialize, Deserialize, Hash, Default, Eq, PartialEq, Debug)]
pub struct NetAddress {
    /// Network byte-order ipv6 address, or ipv4-mapped ipv6 address
    pub address: [u16; 8],
    /// Network port
    pub port: u16,
}

const ONION: [u16; 3] = [0xFD87, 0xD87E, 0xEB43];

impl NetAddress {
    /// Create an address message for a socket
    pub fn new(socket: &SocketAddr) -> NetAddress {
        let (address, port) = match socket {
            &SocketAddr::V4(ref addr) => (addr.ip().to_ipv6_mapped().segments(), addr.port()),
            &SocketAddr::V6(ref addr) => (addr.ip().segments(), addr.port())
        };
        NetAddress { address: address, port: port }
    }


    pub fn socket_address(&self) -> Result<SocketAddr, Error> {
        let addr = &self.address;
        if addr[0..3] == ONION[0..3] {
            return Err(Error::IO(io::Error::from(io::ErrorKind::AddrNotAvailable)));
        }
        let ipv6 = Ipv6Addr::new(
            addr[0], addr[1], addr[2], addr[3],
            addr[4], addr[5], addr[6], addr[7],
        );
        if let Some(ipv4) = ipv6.to_ipv4() {
            Ok(SocketAddr::V4(SocketAddrV4::new(ipv4, self.port)))
        } else {
            Ok(SocketAddr::V6(SocketAddrV6::new(ipv6, self.port, 0, 0)))
        }
    }

    pub fn to_string(&self) -> Result<String, Error> {
        Ok(format!("{}", self.socket_address()?))
    }

    pub fn from_str(s: &str) -> Result<NetAddress, Error> {
        let (address, port) = match SocketAddr::from_str(s)? {
            SocketAddr::V4(ref addr) => (addr.ip().to_ipv6_mapped().segments(), addr.port()),
            SocketAddr::V6(ref addr) => (addr.ip().segments(), addr.port())
        };
        Ok(NetAddress { address, port })
    }
}
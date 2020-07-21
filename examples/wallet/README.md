Example Wallet
==============

This example wallet can be used to test the bdk library.

## Build and Run with Testnet
   
1. Build wallet
   
   ```
   cargo build --examples
   ```

1. Get wallet cli help
 
   ```
   cargo run --example wallet -- -h
   ```

1. Create new wallet

   ```
   cargo run --example wallet -- -p testpass
   ```

1. Check log file to see what's going on

   ```
   tail -f testnet/wallet.log 
   ```
   
1. Once wallet is started, use "help" command from >> prompt to get list of sub-commands.

1. delete testnet data directory if no longer needed
   
   ```
   rm -rf testnet 
   ```
   
## Regtest Testing

1. Clone [bitcoin-regtest-box project](https://github.com/bitcoindevkit/bitcoin-regtest-box) and follow
   [README.md](https://github.com/bitcoindevkit/bitcoin-regtest-box/blob/master/README.md) instructions to start 
   localhost regtest bitcoind nodes.
   
1. Create new regtest wallet
   
   ```
   cargo run --example wallet -- -n regtest -p testpass -a 127.0.0.1:18444
   ```

1. Check log file to see what's going on

   ```
   tail -f regtest/wallet.log 
   ```
   
1. delete regtest data directory if no longer needed
   
   ```
   rm -rf regtest 
   ```
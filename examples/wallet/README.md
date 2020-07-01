Example Wallet
==============

This example wallet can be used to test the bdk library.

## Build and Run
   
1. Build wallet
   
   ```
   cd examples/wallet
   cargo build --examples
   ```

1. Get wallet cli help
 
   ```
   cargo run --example wallet -- -h
   ```

1. Create new wallet

   ```
   cargo run --example wallet -- -p testpw
   ```

1. Check log file to see what's going on

   ```
   tail -f testnet/wallet.log 
   ```
   
1. Once wallet is started, use "help" command from >> prompt to get list of sub-commands.

## REGTEST Testing

1. Clone [bitcoin-regtest-box project](https://github.com/bitcoindevkit/bitcoin-regtest-box) and follow
   [README.md](https://github.com/bitcoindevkit/bitcoin-regtest-box/blob/master/README.md) instructions to start 
   localhost REGTEST bitcoind nodes.
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
   
1. Once wallet is started, use the `help` command from >> prompt to get a list of sub-commands.

1. delete testnet data directory if no longer needed
   
   ```
   rm -rf testnet 
   ```
   
## Regtest Testing

1. The 🍣 [Nigiri CLI](https://github.com/vulpemventures/nigiri) tool can be used to spin-up a complete `regtest` 
   development environment that includes a `bitcoin` node, a Blockstream `electrs` explorer and the 
   [`esplora`](https://github.com/blockstream/esplora) web-app to visualize blocks and transactions in the browser.
   
   First install [Docker-Desktop](https://www.docker.com/products/docker-desktop) on your machine. Then see the 
   [Nigiri CLI README.md](https://github.com/vulpemventures/nigiri/blob/master/README.md) file to install via prebuilt 
   binaries or from the project source.
   
1. Create new regtest wallet
   
   ```
   cargo run --example wallet -- -n regtest -p testpass -a 127.0.0.1:18432
   ```

1. Use the Nigiri Chopsticks API to create a new block and send the reward to `<DEPOSIT ADDRESS>`

   ```
   curl -X POST --data '{"address": "<DEPOSIT ADDRESS>"}' http://localhost:3000/faucet
   ```
   
1. Or you can connect to the Nigiri created bitcoind and use the containers bitcoin-cli to generate multiple bocks and 
   send the reward to `<DEPOSIT ADDRESS>`
   
   ```$xslt
   docker exec -it resources_bitcoin_1 bitcoin-cli -regtest -rpcport=19001 -rpcuser=admin1 -rpcpassword=123 generatetoaddress 100 <DEPOSIT ADDRESS>
   ```
   
1. Or if you have bitcoin-cli installed on your local OS outside the Nigiri container you can use it to tell the Nigiri 
   created bitcoind to generate multiple blocks and send the rewards to `<DEPOSIT ADDRESS>`
   
   ```
   bitcoin-cli -regtest -rpcport=18433 -rpcuser=admin1 -rpcpassword=123 generatetoaddress 100 <DEPOSIT ADDRESS>
   ```

1. Check wallet log file to see what's going on

   ```
   tail -f regtest/wallet.log 
   ```
   
1. delete the ./regtest data directory when no longer needed
   
   ```
   rm -rf regtest 
   ```
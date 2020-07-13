Bitcoin Development Kit
=======================

This library combines rust-bitcoin and rust-wallet libraries to provide basic functionality for interacting with the 
bitcoin network.

## Setup and Build

1. [Install rustup](https://www.rust-lang.org/learn/get-started)

2. Clone the directory using 
   ```
   git clone https://github.com/bitcoindevkit/bdk.git
   ```

1. Clone bitcoindevkit/murmel using
   ```
   git clone https://github.com/bitcoindevkit/murmel.git
   ```

   This is a temporary step until required updates to murmel can be pulled into to the main 
   [rust-bitcoin/murmel](https://github.com/rust-bitcoin/murmel) repository. 

1. Install rust targets (if not already installed)
   
   Android: 
      ```
      rustup target add x86_64-apple-darwin x86_64-unknown-linux-gnu x86_64-linux-android aarch64-linux-android armv7-linux-androideabi i686-linux-android
      ```
      
      iOS:
      ```
      rustup target add aarch64-apple-ios armv7-apple-ios armv7s-apple-ios x86_64-apple-ios i386-apple-ios
      ```
   
1. Install [cargo-ndk](https://docs.rs/crate/cargo-ndk/0.6.1) cargo extension:
   
   Android:
   ```
   cargo install cargo-ndk
   ```

   iOS:
   ```
   cargo install cargo-lipo
   cargo install cbindgen
   ```

1. Install Android Studio and NDK
 
   Open Android Studio -> Tools -> SDK Manager -> SDK Tools -> install "NDK (Side by side)"

1. Set environment variables needed to build rust based library files and
   to run local unit tests. Better yet add these to your `.bash_profile`

    Android (OSX):
    ```
    export ANDROID_HOME=$HOME/Library/Android
    export ANDROID_NDK_HOME=$ANDROID_HOME/sdk/ndk/<ndk version, eg. 21.0.6113669>
    ```
   
    Android (Linux):
    ```
    export ANDROID_HOME=$HOME/Android
    export ANDROID_NDK_HOME=$ANDROID_HOME/Sdk/ndk/<ndk version, eg. 21.0.6113669>
    ```

    iOS (OSX):
    ```
    ## if this fails:
    xcrun -k --sdk iphoneos --show-sdk-path
    ## run this:
    sudo xcode-select --switch /Applications/Xcode.app
    ```

1. Set environment variables needed to build Bitcoin C++ library files. This will be unnecessary after [fix](https://github.com/bbqsrc/cargo-ndk/pull/7) to [cargo-ndk](https://docs.rs/crate/cargo-ndk/0.6.1).

   Android (OSX) 
   ```
   export CXX_x86_64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/x86_64-linux-android30-clang++
   export CXX_aarch64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android30-clang++
   export CXX_armv7_linux_androideabi=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi30-clang++
   export CXX_i686_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/i686-linux-android30-clang++
   ```
   
   Android (Linux)
   ```
   export CXX_x86_64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android30-clang++
   export CXX_aarch64_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang++
   export CXX_armv7_linux_androideabi=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi30-clang++
   export CXX_i686_linux_android=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android30-clang++
   ```
   

1. Build Rust library files for all target platform OS architectures:
    
   ```
   cd bdk
   ./build-lib.sh
   ```
1. Check if all the tests are passing
    
   ```
   cargo test --features android
   cargo test --features java
   ```
   
## REGTEST Testing

1. Clone [bitcoin-regtest-box project](https://github.com/bitcoindevkit/bitcoin-regtest-box) and follow
   [README.md](https://github.com/bitcoindevkit/bitcoin-regtest-box/blob/master/README.md) instructions to start 
   localhost REGTEST bitcoind nodes.
   
   Note: regtest-box only checks against bitcoin-core for now. The same will be updated later for bdk.

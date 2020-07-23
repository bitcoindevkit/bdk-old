Bitcoin Development Kit
=======================

This library combines rust-bitcoin, murmel, and rust-wallet libraries to provide basic functionality for interacting with the 
bitcoin network. The library can be used in an android mobile app by including the optional jni module.

## Setup and Build

1. [Install rustup](https://www.rust-lang.org/learn/get-started)

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
   ./build-lib.sh
   ```
   
## REGTEST Testing

The 🍣 [Nigiri CLI](https://github.com/vulpemventures/nigiri) tool can be used to spin-up a complete `regtest` 
development environment that includes a `bitcoin` node, a Blockstream `electrs` explorer and the 
[`esplora`](https://github.com/blockstream/esplora) web-app to visualize blocks and transactions in the browser.

First install [Docker-Desktop](https://www.docker.com/products/docker-desktop) on your machine. Then see the 
[Nigiri CLI README.md](https://github.com/vulpemventures/nigiri/blob/master/README.md) file to install via prebuilt binaries or from the 
 project source.


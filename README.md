BTC Development Kit
===================

This library combines rust-bitcoin and rust-wallet libraries to provide basic functionality for interacting with the 
bitcoin network.

## Setup and Build

1. Install rust targets (if not already installed)
   
   Android: 
      ```
      rustup target add x86_64-linux-android aarch64-linux-android armv7-linux-androideabi i686-linux-android
      ```
      
      iOS:
      ```
      rustup target add aarch64-apple-ios armv7-apple-ios armv7s-apple-ios x86_64-apple-ios i386-apple-ios
      ```
   
3. Install [cargo-ndk](https://docs.rs/crate/cargo-ndk/0.6.1) cargo extension:
   
   Android:
   ```
   cargo install cargo-ndk
   ```

   iOS:
   ```
   cargo install cargo-lipo
   cargo install cbindgen
   ```

1. Set environment variables needed to build rust based library files and
   to run local unit tests. Better yet add these to your `.bash_profile`

    Android:
    ```
    export ANDROID_HOME=$HOME/Library/Android
    export ANDROID_NDK_HOME=$ANDROID_HOME/sdk/ndk/<ndk version, eg. 21.0.6113669>
    ```

    iOS:
    ```
    ## if this fails:
    xcrun -k --sdk iphoneos --show-sdk-path
    ## run this:
    sudo xcode-select --switch /Applications/Xcode.app
    ```

1. Set environment variables needed to build Bitcoin C++ library files. This will be unnecessary after [fix](https://github.com/bbqsrc/cargo-ndk/pull/7) to [cargo-ndk](https://docs.rs/crate/cargo-ndk/0.6.1).

    ```
    export CXX_x86_64_linux_android=$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/x86_64-linux-android28-clang++
    export CXX_aarch64_linux_android=$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android28-clang++
    export CXX_armv7_linux_androideabi=$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi28-clang++
    export CXX_i686_linux_android=$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/i686-linux-android28-clang++
    ```

1. Build Rust library files for all target platform OS architectures:
    
   ```
   ./build-lib.sh
   ```
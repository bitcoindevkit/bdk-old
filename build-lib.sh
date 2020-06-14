#!/bin/bash
cargo build --target x86_64-apple-darwin --release --features "java"
cargo ndk --platform 28 --target x86_64-linux-android build --release --features "android"
cargo ndk --platform 28 --target aarch64-linux-android build --release --features "android"
cargo ndk --platform 28 --target armv7-linux-androideabi build --release --features "android"
cargo ndk --platform 28 --target i686-linux-android build --release --features "android"
echo built!
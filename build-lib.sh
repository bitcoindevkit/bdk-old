#!/bin/bash

OS=`uname`
if [ "$OS" = "Darwin" ]
then
  echo "building apple darwin x86_64 lib"
  cargo build --target x86_64-apple-darwin --release --features "java"
elif [ "$OS" = "Linux" ]
then
  echo "building linux x86_64 lib"
  cargo build --target x86_64-unknown-linux-gnu --release --features "java"
fi

echo "building android x86_64 lib"
cargo ndk --platform 30 --target x86_64-linux-android build --release --features "android"

echo "building android aarch64 lib"
cargo ndk --platform 30 --target aarch64-linux-android build --release --features "android"

echo "building android armv7 lib"
cargo ndk --platform 30 --target armv7-linux-androideabi build --release --features "android"

echo "building android i686 lib"
cargo ndk --platform 30 --target i686-linux-android build --release --features "android"
echo built!
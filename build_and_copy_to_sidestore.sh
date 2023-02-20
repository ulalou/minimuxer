#!/bin/bash
# You can use `SIDESTORE_REPO="..." ./build_and_copy_to_sidestore.sh` to change the SideStore repo location

if [[ -z "$SIDESTORE_REPO" ]]; then
    SIDESTORE_REPO="../SideStore"
fi

echo "SIDESTORE_REPO: $SIDESTORE_REPO"
set -xe

cargo build --release --target aarch64-apple-ios
cp target/aarch64-apple-ios/release/libminimuxer.a "$SIDESTORE_REPO/Dependencies/prebuilt"
cp minimuxer.h "$SIDESTORE_REPO/Dependencies/prebuilt"

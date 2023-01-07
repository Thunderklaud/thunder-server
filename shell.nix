{ pkgs ? import <nixpkgs> {} }:

let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  rust = pkgs.extend mozillaOverlay;

  # to use the project's rust-toolchain file:
  rustChannel = rust.rustChannelOf { rustToolchain = ./rust-toolchain.toml; };

  # to use a specific nighly:
  #rustChannel = rust.rustChannelOf { date = "2022-04-23"; channel = "nightly"; };

  # to use the latest nightly:
  #rustChannel = rust.rustChannels.nightly

in pkgs.mkShell {
  buildInputs = [
    rustChannel.rust
    rustChannel.rust-src
    rustChannel.cargo
    pkgs.pkg-config
    pkgs.openssl
  ];

  RUST_BACKTRACE = 1;
  RUST_SRC_PATH = "${rustChannel.rust-src}/lib/rustlib/src/rust";
}

let
  rust-overlay = (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/ce79bb52eb023f71a03e88cb36c66f35c6668a95.tar.gz"));
  pkgs = (import <nixpkgs> {
    overlays = [ rust-overlay ];
  });
in
pkgs.mkShell {
  buildInputs = [
    (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
  ];
  packages = with pkgs; [
    lld
  ];
}

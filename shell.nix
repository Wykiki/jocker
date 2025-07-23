let
  rust-overlay = (import (builtins.fetchGit {
    url = "https://github.com/oxalica/rust-overlay.git";
    rev = "ce79bb52eb023f71a03e88cb36c66f35c6668a95";
  }));
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
    pueue
  ];
}

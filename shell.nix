let
  rust-overlay = (import (builtins.fetchGit {
    url = "https://github.com/oxalica/rust-overlay.git";
    rev = "99cc5667eece98bb35dcf35f7e511031a8b7a125";
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
    mold
    pueue
  ];
}

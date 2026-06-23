let
  rust-overlay = import (
    builtins.fetchGit {
      url = "https://github.com/oxalica/rust-overlay.git";
      ref = "master";
    }
  );
  pkgs = import <nixpkgs> {
    overlays = [ rust-overlay ];
  };
in
pkgs.mkShell {
  buildInputs = [
    ((pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
      extensions = [
        "rust-src"
        "rust-analyzer"
      ];
    })
  ];
  packages = with pkgs; [
    mold
    pueue
  ];
}

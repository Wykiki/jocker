{ pkgs ? import (fetchTarball {
  name = "nixos-unstable-2024-11-03";
  url = "https://github.com/nixos/nixpkgs/archive/a86d06940e17a4c236d9ea3cefdd323cad362679.tar.gz";
}) {} }:
pkgs.mkShell {
  packages = with pkgs; [
    clang
    lld
  ];
}

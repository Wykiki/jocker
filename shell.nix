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
  unstable = import <nixos-unstable> { };
  agent-sandbox =
    import
      (fetchTarball {
        url = "https://github.com/archie-judd/agent-sandbox.nix/archive/1a33c51a62ba14f16f59fe0c2febeb63ba73e0be.tar.gz";
        sha256 = "sha256-RF+VyL4XaRzBIr+o3K2nF0CtmzFTG2HpdK39X49ILMs=";
      })
      {
        inherit pkgs;
      };
  claude-sandboxed = agent-sandbox.mkSandbox {
    pkg = unstable.claude-code;
    binName = "claude";
    outName = "claude-sandboxed";
    rwDirs = [
      "$HOME/.claude"
      # "$HOME/.local/share/claude"
      # "$HOME/.cache/claude"
      # "$HOME/.local/state/claude"
      "$HOME/.agents"
      "$HOME/.cargo/registry"
      "/tmp"
    ];
    roDirs = [ ];
    roFiles = [
      "$HOME/.config/git/config"
      "$HOME/.cargo/config.toml"
    ];
    # Bind your host gitconfig read-only for git identity (recommended).
    # Set user.name / user.email on the host first, then uncomment:
    # roFiles = [ "$HOME/.config/git/config" ];
    # (Alternative: set GIT_AUTHOR_* / GIT_COMMITTER_* in env. See README.)
    env = {
      # Pass secrets as shell variable references (e.g. "$TOKEN"), not
      # via builtins.getEnv, so they expand at runtime and stay out of
      # the /nix/store.
      # CLAUDE_CODE_OAUTH_TOKEN = "$CLAUDE_CODE_OAUTH_TOKEN";
      # GITHUB_TOKEN = "$GITHUB_TOKEN";
      # CLAUDE_CODE_OAUTH_TOKEN = "$CLAUDE_CODE_OAUTH_TOKEN";
      CLAUDE_CONFIG_DIR = "$HOME/.claude";
    };
    allowedPackages = agent-sandbox.commonTools ++ [
      pkgs.cargo-nextest
      pkgs.clang
      pkgs.just
      pkgs.lspmux
      unstable.wild
      ((pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
        extensions = [
          "rust-src"
          "rust-analyzer"
        ];
      })
    ];
    allowedDomains = {
      "localhost" = "*";
      "127.0.0.1" = "*";
      "anthropic.com" = "*";
      "claude.com" = "*";
      "crates.io" = "*";
      "*.crates.io" = "*";
      # Private Cargo registry (sparse index + crate downloads).
      "crates.dev-factory.netwo.dev" = [
        "GET"
        "HEAD"
      ];
      "github.com" = [
        "GET"
        "HEAD"
        "POST"
      ];
      "raw.githubusercontent.com" = [
        "GET"
        "HEAD"
      ];
      "api.github.com" = [
        "GET"
        "HEAD"
      ];
      "models.dev" = [ "GET" ];
      "openrouter.ai" = "*";
    };
    allowedLocalPorts = [ ];
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
    clang
    unstable.wild
    pueue
    claude-sandboxed
  ];
}

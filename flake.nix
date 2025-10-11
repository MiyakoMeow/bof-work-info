{
  description = "BOF Work Info - Rust workspace development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        # 使用最新的稳定版 Rust
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };
        
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust 工具链
            rust
            
            # 系统依赖
            pkg-config
            openssl
            curl
            
            # 开发工具
            cargo-watch
            cargo-expand
            cargo-udeps
            cargo-audit
            cargo-outdated
            
          ];

          shellHook = ''
            echo "🚀 BOF Work Info Development Environment"
            echo "======================================"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            
            # 设置环境变量
            export RUST_BACKTRACE=1
            export RUST_LOG=debug
            
            # 确保 cargo 能找到 rust-src
            export RUST_SRC_PATH=${rust}/lib/rustlib/src/rust/library
          '';

          # 设置环境变量
          RUST_BACKTRACE = "1";
          RUST_LOG = "debug";
          RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";
        };
      });
}

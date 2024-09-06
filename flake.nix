{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    rust-overlay.url = "https://flakehub.com/f/oxalica/rust-overlay/*.tar.gz";
    flake-utils.url = "https://flakehub.com/f/numtide/flake-utils/0.1.92.tar.gz";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }: 
   flake-utils.lib.eachDefaultSystem(system:
    let 
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      rust-pkg =  (pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" "rustfmt" "llvm-tools" ];
        targets = [ "aarch64-apple-darwin" "x86_64-unknown-linux-gnu" ];
      });
    in 
    {
      devShells.default = with pkgs; mkShell {
        packages = [ redis rust-pkg ] ++ lib.optionals stdenv.isDarwin [
          libiconv
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];
      };
    });
}


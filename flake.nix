{
  inputs = {
    # To update all inputs: nix flake update --recreate-lock-file
    nixpkgs.url = "github:nixos/nixpkgs/release-21.11";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, naersk, devshell, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;

          overlays = [
            (import rust-overlay)
            devshell.overlay
            naersk.overlay
          ];
        };

        naersk-lib = naersk.lib."${system}".override {
          cargo = pkgs.cargo;
          rustc = pkgs.rustc;
        };

        rust = (pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        });

        crateName = "mzoon";

        buildInputs = with pkgs; [
          rust
          pkgconfig
          openssl
        ];
      in {
        defaultPackage = naersk-lib.buildPackage {
          inherit buildInputs;
          name = crateName;
          pname = crateName;
          root = ./.;
        };

        packages = builtins.foldl' (packages: crateName: packages // {
          ${crateName} = naersk-lib.buildPackage {
            inherit buildInputs;
            name = crateName;
            pname = crateName;
            root = ./examples/${crateName};
          };
        }) {} (builtins.attrNames (builtins.readDir (
          builtins.filterSource (path: type: type == "directory" && builtins.pathExists "${path}/Cargo.lock") ./examples)));

        devShell = pkgs.devshell.mkShell {
          packages = with pkgs; [
            cargo-make
            self.defaultPackage.${system}
          ];
        };
      });
}

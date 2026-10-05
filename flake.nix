{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      systems,
      rust-overlay,
      git-hooks,
      ...
    }:
    let
      forAllSystems = nixpkgs.lib.genAttrs (import systems);
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;

            overlays = [
              rust-overlay.overlays.default
            ];
          };

          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          gitHooks = git-hooks.lib.${system}.run {
            src = ./.;

            package = pkgs.prek;

            default_stages = [ "pre-push" ];

            hooks = {
              rustfmt = {
                enable = true;
                priority = 10;

                settings.check = true;

                packageOverrides = {
                  cargo = rustToolchain;
                  rustfmt = rustToolchain;
                };
              };

              clippy = {
                enable = true;
                priority = 20;
                always_run = true;

                settings = {
                  allFeatures = true;
                  denyWarnings = true;
                  offline = false;
                  extraArgs = "--workspace --all-targets";
                };

                packageOverrides = {
                  cargo = rustToolchain;
                  clippy = rustToolchain;
                };
              };

              tests = {
                enable = true;
                name = "cargo test";
                priority = 30;
                always_run = true;

                entry = "${rustToolchain}/bin/cargo test --workspace --all-features";

                pass_filenames = false;
              };
            };
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain

              pkgs.pkg-config
            ]
            ++ gitHooks.enabledPackages;

            inherit (gitHooks) shellHook;

            RUST_BACKTRACE = "1";
          };
        }
      );
    };
}

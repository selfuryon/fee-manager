{ pkgs, lib, config, ... }:
{
  languages.rust.enable = true;
  packages = builtins.attrValues {
    inherit (pkgs)
      sqlx-cli
      cargo-watch
      pinact
      zizmor
      actionlint
      ;
  };
  env.DATABASE_URL = "postgres://feemanager:feemanager@localhost/feemanager";
  services.postgres = {
    # Same major as the CI service container.
    package = pkgs.postgresql_18;
    enable = true;
    listen_addresses = "127.0.0.1";
    initialDatabases = [
      {
        name = "feemanager";
        user = "feemanager";
        pass = "feemanager";
      }
    ];
  };
  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy = {
      enable = true;
      # Check against the committed .sqlx cache, as CI and the Containerfile
      # do: the hook must not depend on a running postgres, and a query missing
      # from the cache fails here instead of in CI.
      entry = lib.mkForce "env SQLX_OFFLINE=true ${config.git-hooks.hooks.clippy.package}/bin/cargo-clippy clippy --offline --all-targets -- -D warnings";
    };
    actionlint.enable = true;
    zizmor.enable = true;
  };
}

{ pkgs, ... }:
{
  env.DATABASE_URL = "postgres://feemanager:feemanager@localhost/feemanager";
  languages.rust.enable = true;
  packages = [
    pkgs.sqlx-cli
  ];
  services.postgres = {
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
  # pre-commit.hooks = {
  #   # rustfmt.enable = true;
  #   # clippy.enable = true;
  # };
}

{
  description = "Telegram bill Qr code scanner";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/17b62c338f2a0862a58bb6951556beecd98ccda9"; # Stable
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
          {
            devShell = pkgs.mkShell {
              buildInputs = with pkgs; [
                cargo
                rustc
                rustfmt
                rust-analyzer

                pkgconfig
                openssl
              ];
            };
          }
      );
}

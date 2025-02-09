{
	description = "C++ Development Environment";

	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
		flake-utils.url = "github:numtide/flake-utils";
	};

	outputs = { self, nixpkgs, flake-utils }:
		flake-utils.lib.eachDefaultSystem (system:
			let
				pkgs = import nixpkgs { system = system; config.cudaSupport = true; config.allowUnfree = true; };
			in {
				devShells.default = import ./shell.nix { pkgs = pkgs; };
			}
		);
}

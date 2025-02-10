{
	pkgs ? import <nixpkgs> {
		config.cudaSupport = true;
		config.allowUnfree = true;
	}
}:

pkgs.mkShell {
	buildInputs = with pkgs; [
		cargo
		rustc
		rust-analyzer
		openssl
		pkg-config
		xorg.libxcb
	];
}


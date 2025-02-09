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
		openssl
		pkg-config
		xorg.libxcb
	];
}


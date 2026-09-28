{
  description = "Rust devShell";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    
    flake-utils.url = "github:numtide/flake-utils";
    
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
	overlays = [ (import rust-overlay) ];
	pkgs = import nixpkgs { inherit system overlays; };
	rustToolchain = pkgs.rust-bin.stable.latest.default.override {
	  extensions = [
	    "rust-src"
	    "rust-analyzer"
	  ];
	};

      in {
	devShells.default = with pkgs; mkShell {
	  buildInputs = [
            rustToolchain
            pkg-config

	    # Packages for window manager test
	    feh
            rofi
            picom
	    alacritty
	    polybar
	  ];

	  shellHook = ''
            echo "Rust environment activated."
	  '';
	};
      }
    );
}

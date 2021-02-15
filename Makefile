release:
	cargo build --release

release-linux:
	cargo build --target=x86_64-unknown-linux-gnu --release

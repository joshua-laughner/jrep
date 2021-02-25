release: release-static

# By default, the rust std. lib links against the GNU C standard library,
# which cannot be linked statically. That means that if I upload those
# binaries to a computer that doesn't have the necessary GLibC available,
# it won't work, because it needs to dynamically link. MUSL is an alternative
# implementation that is stricter, but allows full static linking.
# C.F. https://doc.rust-lang.org/edition-guide/rust-2018/platform-and-target-support/musl-support-for-fully-static-binaries.html
release-static:
	cargo build --release --target x86_64-unknown-linux-musl

release-local:
	cargo build --release

release-linux:
	cargo build --target=x86_64-unknown-linux-gnu --release

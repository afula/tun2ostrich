ios:
	cargo lipo --release -p ostrich-ffi
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/universal/release/ostrich.h

ios-dev:
	cargo lipo -p ostrich-ffi
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/universal/debug/ostrich.h

ios-opt:
	RUSTFLAGS="-Z strip=symbols" cargo lipo --release --targets aarch64-apple-ios --manifest-path ostrich-ffi/Cargo.toml --no-default-features --features "default-openssl"
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/universal/release/ostrich.h

mac:
	RUSTFLAGS="-Z strip=symbols" cargo lipo --release --targets x86_64-apple-darwin -p ostrich-ffi
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/debug/ostrich.h

mac-m1:
    RUSTFLAGS="-Z strip=symbols" cargo lipo --release --targets aarch64-apple-darwin -p ostrich-ffi
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/debug/ostrich.h

lib:
	cargo build -p ostrich-ffi --release
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/release/ostrich.h

lib-dev:
	cargo build -p ostrich-ffi
	cbindgen --config ostrich-ffi/cbindgen.toml ostrich-ffi/src/lib.rs > target/debug/ostrich.h

local:
	cargo build -p ostrich-bin --release

local-dev:
	cargo build -p ostrich-bin

mipsel:
	./misc/build_cross.sh mipsel-unknown-linux-musl

mips:
	./misc/build_cross.sh mips-unknown-linux-musl

test:
	cargo test -p ostrich -- --nocapture

# Force a re-generation of protobuf files.
proto-gen:
	touch ostrich/build.rs
	PROTO_GEN=1 cargo build -p ostrich

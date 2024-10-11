.PHONY: coverage doc

TARGET_DIR=~/.cache/rust-target
DOC_DIR=/mnt/c/Users/briot/Desktop

coverage: 
	rm -rf ${TARGET_DIR}/coverage
	CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test
	grcov . --binary-path ${TARGET_DIR}/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o ${TARGET_DIR}/coverage/html
	cp -R ${TARGET_DIR}/coverage/html ${DOC_DIR}/

doc:
	cargo doc
	cp -R ${TARGET_DIR}/doc/rust_intervals/* ${DOC_DIR}/doc/rust_intervals/

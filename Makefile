.PHONY: coverage

TARGET_DIR=~/.cache/rust-target
DOC_DIR=/mnt/c/Users/briot/Desktop

coverage:
	cargo make cov
	cp -R ${TARGET_DIR}/llvm-cov/html ${DOC_DIR}/

doc:
	cargo make doc
	cp -R ${TARGET_DIR}/doc ${DOC_DIR}/

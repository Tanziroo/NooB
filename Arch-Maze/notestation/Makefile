.PHONY: all build test fmt clippy smoke clean help

help:
	@echo "make build   - build scribe (release)"
	@echo "make test    - cargo tests + python smoke tests"
	@echo "make fmt     - check rust formatting"
	@echo "make clippy  - lint rust (deny warnings)"
	@echo "make smoke   - run the python tool smoke tests"
	@echo "make all     - fmt + clippy + build + test (what CI runs)"

build:
	cd scribe && cargo build --release

test:
	cd scribe && cargo test --release
	bash tests/smoke.sh

fmt:
	cd scribe && cargo fmt --check

clippy:
	cd scribe && cargo clippy --release -- -D warnings

smoke:
	bash tests/smoke.sh

all: fmt clippy build test

clean:
	cd scribe && cargo clean

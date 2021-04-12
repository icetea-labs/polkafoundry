.PHONY: init
init:
	./scripts/init.sh

.PHONY: check
check:
	SKIP_WASM_BUILD=1 cargo check --release

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --release --all

.PHONY: run
run:
	 cargo run -- --dev --tmp --start-dev --validator  --sealing=Manual

.PHONY: build
build:
	 cargo build --release

.PHONY: spec
spec:
	./target/release/polkafoundry build-spec --disable-default-bootnode --chain local > tests/specs/polka-spec.json

.PHONY: copy
copy:
	cp ./target/release/polkafoundry ./bin


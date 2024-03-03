.PHONY: testdata apollo apollo-release synchs synchs-release sink-exp sink-exp-release

.PHONY: release
release:
	cargo build --all --release

.PHONY: debug
debug:
	cargo build --all

.PHONY: tools
tools:
	cargo build --package=genconfig --release
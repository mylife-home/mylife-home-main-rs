# Cargo targets shared by every component Makefile and by the root one.
#
# The crates form a single workspace, so each target covers all of them and
# gives the same result whatever directory make was started from.

CARGO ?= cargo

.PHONY: fmt fmt-check lint test

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

# Clippy drives the compiler, so this reports rustc warnings besides its own.
lint:
	$(CARGO) clippy --workspace --all-targets

test:
	$(CARGO) test --workspace

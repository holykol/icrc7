CARGO_TARGET_DIR ?= ./target
export CARGO_INCREMENTAL = 0
export RUSTFLAGS = -Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off

cov:
	echo ${CARGO_INCREMENTAL}
	cargo +nightly test
	grcov ${CARGO_TARGET_DIR}/debug/ -s . -t html --llvm --branch --ignore-not-existing -o ${CARGO_TARGET_DIR}/debug/coverage/


showcov:
	python -m http.server 8899 --directory ${CARGO_TARGET_DIR}/debug/coverage/
BIN_FILE := alfred-ai-callback
LINT_PARAMS := $(shell cat .lints | cut -f1 -d"#" | tr '\n' ' ')

build:
	cargo build --bin alfred-openai-chat --bin alfred-openai-stt --bin alfred-openai-tts
build-release:
	cargo build --release --bin alfred-openai-chat --bin alfred-openai-stt --bin alfred-openai-tts
aarch64:
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-chat --bin alfred-openai-stt --bin alfred-openai-tts

install: clean-bin build
	mkdir bin
	cp target/debug/alfred-openai-chat bin/
	cp target/debug/alfred-openai-stt bin/
	cp target/debug/alfred-openai-tts bin/
install-aarch64: clean-bin aarch64
	mkdir bin
	cp target/aarch64-unknown-linux-gnu/release/alfred-openai-chat bin/
	cp target/aarch64-unknown-linux-gnu/release/alfred-openai-stt bin/
	cp target/aarch64-unknown-linux-gnu/release/alfred-openai-tts bin/

clean: clean-target clean-bin
clean-target:
	rm -rf target
clean-bin:
	rm -rf bin
clippy:
	cargo clippy --all-targets --all-features -- -D warnings $(LINT_PARAMS)

clippy-fix:
	__CARGO_FIX_YOLO=1 cargo clippy --fix --allow-staged --all-targets --all-features -- -D warnings $(LINT_PARAMS)

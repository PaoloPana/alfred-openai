BIN_FILE := alfred-ai-callback

build:
	cargo build --bin alfred-openai-chat
	cargo build --bin alfred-openai-stt
	cargo build --bin alfred-openai-tts
build-release:
	cargo build --bin alfred-openai-chat --release
	cargo build --bin alfred-openai-stt --release
	cargo build --bin alfred-openai-tts --release
aarch64:
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-chat
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-stt
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-tts

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

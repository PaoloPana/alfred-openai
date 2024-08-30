build:
	cargo build --bin alfred-openai-chat
	cargo build --bin alfred-openai-stt
	cargo build --bin alfred-openai-tts
build_release:
	cargo build --bin alfred-openai-chat --release
	cargo build --bin alfred-openai-stt --release
	cargo build --bin alfred-openai-tts --release
aarch64:
	if [ -d "bin" ] ; then rm -rf bin/; fi
	mkdir bin
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-chat
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-stt
	cross build --release --target aarch64-unknown-linux-gnu --bin alfred-openai-tts
	cp target/aarch64-unknown-linux-gnu/release/alfred-openai-chat bin/
	cp target/aarch64-unknown-linux-gnu/release/alfred-openai-stt bin/
	cp target/aarch64-unknown-linux-gnu/release/alfred-openai-tts bin/

# Release builds and dev runs for each front end. The Dioxus targets need the
# Dioxus CLI (`dx`, 0.7); the Iced app is a plain Cargo workspace member.

.PHONY: desktop-dioxus-release desktop-iced-release web-release desktop-dioxus desktop-iced web test run-script

# -> target/dx/tabkeeper/release/linux/app/
desktop-dioxus-release:
	dx build --desktop --release

# -> target/release/tabkeeper-iced
desktop-iced-release:
	cargo build --release -p tabkeeper-iced

# -> target/dx/tabkeeper/release/web/public/
web-release:
	dx build --web --release

desktop-dioxus:
	dx serve --platform desktop

desktop-iced:
	cargo run -p tabkeeper-iced

web:
	dx serve --platform web

test:
	cargo test --workspace

run-script:
	cargo run --example run_script ${SCRIPT} || echo Usage: SCRIPT=file make run-script

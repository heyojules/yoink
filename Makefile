HOTKEY  ?= cmd+shift+4
# A real certificate gives a stable code identity, so the Screen Recording grant
# survives rebuilds (ad-hoc signatures change on every build and lose it).
SIGN_ID ?= -

LABEL  := local.yoink
BIN    := $(HOME)/.local/bin/yoink
PLIST  := $(HOME)/Library/LaunchAgents/$(LABEL).plist
LOG    := $(HOME)/Library/Logs/yoink.log
DOMAIN := gui/$(shell id -u)

.PHONY: build test install uninstall restart logs

build:
	cargo build --release

test:
	cargo test

install: build
	install -d $(dir $(BIN)) $(dir $(PLIST))
	codesign --force --sign "$(SIGN_ID)" --identifier $(LABEL) target/release/yoink
	install -m 755 target/release/yoink $(BIN)
	sed -e 's|@LABEL@|$(LABEL)|' -e 's|@BIN@|$(BIN)|' -e 's|@HOTKEY@|$(HOTKEY)|' -e 's|@LOG@|$(LOG)|' \
		launchd.plist > $(PLIST)
	@launchctl bootout $(DOMAIN)/$(LABEL) 2>/dev/null || true
	@# bootout is async; bootstrap fails (error 5) until the old job is gone
	@while launchctl print $(DOMAIN)/$(LABEL) >/dev/null 2>&1; do sleep 0.1; done
	launchctl bootstrap $(DOMAIN) $(PLIST)
	@echo "yoink installed — press $(HOTKEY). Logs: $(LOG)"

uninstall:
	@launchctl bootout $(DOMAIN)/$(LABEL) 2>/dev/null || true
	rm -f $(PLIST) $(BIN)

restart:
	launchctl kickstart -k $(DOMAIN)/$(LABEL)

logs:
	tail -f $(LOG)

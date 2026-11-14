# Single install recipe, used by `sudo make install` on any distro.
PREFIX ?= /usr/local
DESTDIR ?=

all:
	cargo build --release --locked -p inkuno

install:
	install -Dm755 target/release/inkuno $(DESTDIR)$(PREFIX)/bin/inkuno
	install -Dm644 packaging/linux/com.inkuno.Inkuno.desktop $(DESTDIR)$(PREFIX)/share/applications/com.inkuno.Inkuno.desktop
	install -Dm644 packaging/linux/com.inkuno.Inkuno.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/com.inkuno.Inkuno.svg
	install -Dm644 packaging/linux/inkuno.metainfo.xml $(DESTDIR)$(PREFIX)/share/metainfo/inkuno.metainfo.xml
	install -Dm644 LICENSE $(DESTDIR)$(PREFIX)/share/licenses/inkuno/LICENSE

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/inkuno \
	  $(DESTDIR)$(PREFIX)/share/applications/com.inkuno.Inkuno.desktop \
	  $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/com.inkuno.Inkuno.svg \
	  $(DESTDIR)$(PREFIX)/share/metainfo/inkuno.metainfo.xml
	rm -rf $(DESTDIR)$(PREFIX)/share/licenses/inkuno

test:
	cargo test --locked

# The user guide: the shortcuts page is written from the help panel, then the book is built
# into docs/user/book/ (needs mdbook: `cargo install mdbook`).
docs:
	INKUNO_WRITE_DOCS=1 cargo test --locked shortcuts_page_is_current
	mdbook build docs/user

.PHONY: all install uninstall test docs

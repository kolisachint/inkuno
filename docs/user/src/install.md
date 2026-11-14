# Install

| Your system | How |
|---|---|
| Linux, x86_64 | [Ready-made binary](#linux) |
| macOS 11 or later | [Disk image](#macos) |
| Ubuntu, Linux Mint, Debian | [Build from source](#ubuntu-linux-mint-debian) |
| Fedora | [Build from source](#fedora) |
| Another Linux distribution | [Build from source](#other-linux-distributions) |

macOS and Linux have a ready-made download. Everywhere else, Inkuno is compiled on your machine. The first build takes a few minutes.

Compiling needs a recent stable Rust. If `rustc --version` shows less than 1.98, install Rust with [rustup](https://rustup.rs).

## Linux

Every release ships a ready-made `inkuno` binary for x86_64. Unpack it somewhere on your `PATH`:

```bash
curl -LO https://github.com/kolisachint/inkuno/releases/latest/download/inkuno-linux-x86_64.tar.gz
tar -xzf inkuno-linux-x86_64.tar.gz
sudo install inkuno /usr/local/bin/
```

## macOS

Download [`inkuno-macos.dmg`](https://github.com/kolisachint/inkuno/releases/latest/download/inkuno-macos.dmg). It runs on Apple Silicon and Intel. Open it and drag Inkuno to Applications.

The app is not notarized, because that needs a paid Apple Developer account. So macOS blocks the first launch:

1. Open **System Settings**, then **Privacy & Security**.
2. Scroll down and click **Open Anyway**.

Or run this once in a terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Inkuno.app
```

**To compile it yourself**, you need Xcode (the full app, for the Metal shader compiler) and Rust:

```bash
git clone https://github.com/kolisachint/inkuno.git
cd inkuno
cargo install --path crates/inkuno --locked
```

This installs the `inkuno` command in `~/.cargo/bin`, without the `.app` bundle. Start it from a terminal.

## Ubuntu, Linux Mint, Debian

1. Install the build tools and libraries:

   ```bash
   sudo apt install build-essential pkg-config libfontconfig-dev libfreetype-dev \
     libwayland-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libvulkan1 \
     fonts-noto-color-emoji
   ```

2. Install Rust with [rustup](https://rustup.rs). The `cargo` from apt is usually too old.

3. Build and install:

   ```bash
   git clone https://github.com/kolisachint/inkuno.git
   cd inkuno
   make
   sudo make install
   ```

## Fedora

Follow the [steps above](#ubuntu-linux-mint-debian) with these packages. Their names have not been tested.

```bash
sudo dnf install gcc pkgconf-pkg-config fontconfig-devel freetype-devel \
  wayland-devel libxcb-devel libxkbcommon-devel libxkbcommon-x11-devel vulkan-loader \
  google-noto-color-emoji-fonts
```

## Other Linux distributions

You need:

- the development files of fontconfig, freetype, Wayland, libxcb and libxkbcommon (with its X11 part);
- a Vulkan driver for your GPU;
- an emoji font, such as Noto Color Emoji, for 🎉 in notes and names;
- Rust.

Then:

```bash
make                    # cargo build --release --locked
sudo make install       # PREFIX=/usr/local by default, PREFIX=/usr for a system-wide install
```

`make install` adds the `inkuno` command, an entry in your application menu, and its icon. `sudo make uninstall` removes them.

## Upgrading

Inkuno [never checks for updates](updates.md): fetch the new release yourself, the same way you installed the app.

| Installed with | Upgrade |
|---|---|
| Linux, ready-made binary | download the new archive and put the binary on your `PATH` again |
| macOS, `.dmg` | download the new `.dmg` and drop Inkuno on Applications again |
| `make install` | `git pull`, then `make` and `sudo make install` |
| `cargo install` | `git pull`, then `cargo install --path crates/inkuno --locked` |

## Coming from encre

Inkuno was called *encre* until version 0.1.4. Your notes are untouched. Settings saved under the old name are still read, and written under the new one the next time they change.

| Installed encre with | Before upgrading |
|---|---|
| `make install` | `sudo make uninstall` in the old checkout, before `git pull` |
| `cargo install` | `cargo uninstall encre` |

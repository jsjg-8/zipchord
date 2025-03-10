# PKGBUILD
pkgname=zipchord
pkgver=0.1.0
pkgrel=1
pkgdesc="Keyboard chording system for Wayland"
arch=('x86_64')
url="https://github.com"
license=('MIT')
depends=('ydotool' 'evtest')  # Include any runtime dependencies
makedepends=('cargo' 'systemd')  # Include build dependencies like systemd
source=("$pkgname-$pkgver.tar.gz" "install.sh" "systemd/zipchord.service" "dictionaries/*")
sha256sums=('SKIP' 'SKIP' 'SKIP' 'SKIP')  # Replace with actual hashes

build() {
    cd "$pkgname-$pkgver"
    cargo build --release
}

package() {
    cd "$pkgname-$pkgver"
    
    # Install binary
    install -Dm755 "target/release/zipchord" "$pkgdir/usr/bin/zipchord"

    # Install the setup script
    install -Dm755 "$srcdir/install.sh" "$pkgdir/usr/bin/zipchord-setup"
    
    # Install systemd service file
    install -Dm644 "$srcdir/systemd/zipchord.service" "$pkgdir/usr/lib/systemd/user/zipchord.service"
    
    # Ensure config directory and library path exist
    install -d "$pkgdir${XDG_CONFIG_HOME:-$HOME/.config}/chords"
    install -d "$pkgdir${XDG_DATA_HOME:-$HOME/.local/share}/chords/lib"

    # Copy dictionaries to the user's libraries folder
    for dict in "$srcdir/dictionaries"/*; do
        install -Dm644 "$dict" "$pkgdir${XDG_DATA_HOME:-$HOME/.local/share}/chords/lib/$(basename "$dict")"
    done
}

pkgname=initium
pkgver=0.1.0
pkgrel=1
pkgdesc='Configurable early boot flow and Linux boot and shutdown interfaces'
arch=('x86_64' 'aarch64')
url='https://github.com/AlexKletn/Initium'
license=('GPL-2.0-only')
depends=('glibc' 'libgcc' 'libdrm>=2.4.119' 'systemd')
makedepends=('rust' 'clang-libs')
options=('!debug' '!lto')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo build --frozen --release --workspace --bins --target-dir "$srcdir/target"
}

check() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo test --frozen --workspace --target-dir "$srcdir/target"
}

package() {
    local binary document unit
    cd "$srcdir/$pkgname-$pkgver"

    for binary in initium initium-gui-preview; do
        install -Dm755 "$srcdir/target/release/$binary" "$pkgdir/usr/bin/$binary"
    done
    install -Dm755 "$srcdir/target/release/initium-shutdown" "$pkgdir/usr/lib/initium/initium-shutdown"

    for unit in systemd/*.service; do
        install -Dm644 "$unit" "$pkgdir/usr/lib/systemd/system/${unit##*/}"
    done

    for document in README README.ru CONTRIBUTING CONTRIBUTING.ru CODE_QUALITY CODE_QUALITY.ru COMMIT_CONVENTIONS COMMIT_CONVENTIONS.ru; do
        install -Dm644 "$document.md" "$pkgdir/usr/share/doc/$pkgname/$document.md"
    done
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}

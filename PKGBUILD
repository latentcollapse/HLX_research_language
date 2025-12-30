# Maintainer: Matt Cohn <latentcollapse@gmail.com>
pkgname=hlx-compiler-git
pkgver=0.1.0
pkgrel=1
pkgdesc="Deterministic GPU execution via Vulkan/SPIR-V"
arch=('x86_64')
url="https://github.com/latentcollapse/hlx-compiler"
license=('Apache')
depends=('vulkan-icd-loader' 'shaderc')
makedepends=('git' 'rust' 'cargo' 'python')
provides=('hlx-compiler')
conflicts=('hlx-compiler')
source=("git+$url")
md5sums=('SKIP')

pkgver() {
  cd "$srcdir/hlx-compiler"
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
  cd "$srcdir/hlx-compiler"
  # Compile shaders first
  python3 scripts/compile_shaders.py
  # Build binaries
  cargo build --release --locked --all-features
}

package() {
  cd "$srcdir/hlx-compiler"
  # Install binary
  install -Dm755 target/release/train_transformer_full "$pkgdir/usr/bin/hlx-train"
  install -Dm755 target/release/hlx_shader_compiler "$pkgdir/usr/bin/hlx-compile"
  
  # Install license
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  
  # Install README
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}

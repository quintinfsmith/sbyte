import os
import toml

def build_pkgbuild(manifest):
    output = """
# Maintainer: %s
pkgname=%s
pkgver=%s
pkgrel=1
epoch=
pkgdesc="%s"
arch=('x86_64')
url=""
license=('GPL')
groups=()
depends=('wrecked>=0.1.0')
makedepends=()
checkdepends=()
optdepends=()
provides=()
conflicts=()
replaces=()
backup=()
options=()
install=
changelog=
source=("$pkgname-$pkgver.tar.gz")
noextract=()
validpgpkeys=()

prepare() {
    cd "$pkgname-$pkgver"
}

build() {
    cd "$pkgname-$pkgver"
}

check() {
    cd "$pkgname-$pkgver"
}

package() {
    cd "$pkgname-$pkgver"
    chmod +x ./usr/bin/%s
    mv ./usr/ "$pkgdir/"
    mv ./etc/ "$pkgdir/"
}
""" % (
    ",".join(manifest['package']['authors']),
    manifest['package']['name'],
    manifest['package']['version'],
    manifest['package']['description'],

    manifest['package']['name']
)

    return output


manifest = toml.load("Cargo.toml")
name = manifest['package']['name']
version = manifest['package']['version']
description = manifest['package']['description']
os.mkdir(name)
os.chdir(name);

folder = "%s-%s" % (name, version)
os.system('cargo build --release')
os.system('rm target/release/build -rf')
os.system("mkdir %s/etc/%s/ -p" % (folder, name))

# SPECIFIC TO SBYTE
os.system("cp ../sbyterc %s/etc/%s/" % (folder, name))


os.system("mkdir %s/usr/lib/%s -p" % (folder, name))
os.system("cp ../target/release/* %s/usr/lib/%s/ -r" % (folder, name))

os.system("mkdir %s/usr/bin/ -p" % folder)
with open("%s/usr/bin/%s" % (folder, name), "w") as fp:
    fp.write("""
        export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/usr/lib/%s
        /usr/lib/%s/%s "$@"
    """ % (name, name, name))

os.system("tar --create --file \"%s.tar.gz\" %s" % (folder, folder))
os.system("rm \"%s\" -rf" % folder)

with open("PKGBUILD", "w") as fp:
    fp.write(build_pkgbuild(manifest))
os.system("makepkg -g -f -p PKGBUILD >> PKGBUILD")

os.system("rm src -rf")

os.chdir("../")
os.system("tar --create --file \"%s-dist.tar.gz\" %s/*" % (folder, name))
os.system("rm %s -rf" % name)

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
url="%s"
license=('GPL3')
groups=()
depends=()
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
    cd "$pkgname"
}

build() {
    cd "$pkgname"
}

check() {
    cd "$pkgname"
}

package() {
    cd "$pkgname"
    chmod +x ./usr/bin/%s
    mv ./usr/ "$pkgdir/"
    mv ./etc/ "$pkgdir/"
}
""" % (
    ",".join(manifest['package']['authors']),
    manifest['package']['name'],
    manifest['package']['version'],
    manifest['package']['description'],
    manifest['package']['repository'],

    manifest['package']['name']
)

    return output

def build_control(manifest):

    output = """
Package: %s
Version: %s
Section: custom
Priority: optional
Architecture: x86-64
Essential: no
Installed-Size: 1024
Maintainer: %s
Description: %s

""" % (
        manifest['package']['name'],
        manifest['package']['version'],
        ",".join(manifest['package']['authors']),
        manifest['package']['description']
    )
    return output


manifest = toml.load("Cargo.toml")
name = manifest['package']['name']
version = manifest['package']['version']
description = manifest['package']['description']

if os.path.isdir(name):
    os.rmdir(name)
os.mkdir(name)
os.chdir(name)

folder = name
os.system('cargo build --release --target x86_64-unknown-linux-musl')

os.system("mkdir %s/etc/%s/ -p" % (folder, name))
os.system("mkdir %s/usr/bin/ -p" % (folder))
# SPECIFIC TO SBYTE
os.system("cp ../sbyterc %s/etc/%s/" % (folder, name))

os.system("cp ../target/x86_64-unknown-linux-musl/release/%s %s/usr/bin/" % (name, folder))

#dpkg
os.mkdir("%s/DEBIAN" % folder)
with open("%s/DEBIAN/control" % folder, "w") as fp:
    fp.write(build_control(manifest))

copyright = """Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: %s
Upstream-Contact: %s
Source: %s

Files: *
Copyright:
License: GPL-3
""" % (
    name,
    manifest['package']['authors'][0],
    manifest['package']['repository']
)
with open("%s/DEBIAN/copyright" % folder, "w") as fp:
    fp.write(copyright)

os.system("chmod -R 755 ./*")
os.system("dpkg-deb --build %s" % name)
os.system("rm %s/DEBIAN -rf")
os.system("mv %s.deb ../" % name)

# pacman (Needs to be run 2nd)
os.system("tar --create --file \"%s-%s.tar.gz\" %s" % (name, version, name))
os.system("rm \"%s\" -rf" % folder)

with open("PKGBUILD", "w") as fp:
    fp.write(build_pkgbuild(manifest))
os.system("makepkg -g -f -p PKGBUILD >> PKGBUILD")


os.system("rm src -rf")

os.chdir("../")
os.system("tar --create --file \"%s.tar.gz\" %s/*" % (folder, name))
os.system("rm %s -rf" % name)

#!/usr/bin/env python3
"""Verify OpenWrt release packages are valid native installable formats.

Background (see docs/refactor/journal-release-assets.md):
  - .ipk from current OpenWrt SDK is gzip(tar(debian-binary, control.tar.gz,
    data.tar.gz)) -- NOT an ar archive. `ar t` on an .ipk fails by design
    (ipkg-build: tar | gzip > pkg.ipk, openwrt commit 7a732213).
  - .apk from OpenWrt 25.12 / snapshot SDKs is APK v3 ("ADB" container,
    Alpine apk-tools v3). `gzip -t` fails by design -- the body is a raw
    deflate stream (mode byte 'd'), not a gzip stream.
  Both are installable by opkg / apk on the device; local ar/gzip checks
  produce false negatives. This script validates against the real specs.

Usage: verify_release_packages.py <dir-with-ipk-and-apk>
Exit code 0 = all valid, 1 = at least one invalid.
"""
import gzip
import io
import os
import sys
import tarfile
import zlib


def verify_ipk(data: bytes) -> None:
    raw = gzip.decompress(data)
    tf = tarfile.open(fileobj=io.BytesIO(raw))
    names = set(tf.getnames())
    need = {"./debian-binary", "./control.tar.gz", "./data.tar.gz"}
    missing = need - names
    if missing:
        raise ValueError(f"ipk missing members: {sorted(missing)}")
    ctrl = tf.extractfile("./control.tar.gz").read()
    cnames = tarfile.open(fileobj=io.BytesIO(ctrl)).getnames()
    if "./control" not in cnames:
        raise ValueError(f"ipk control.tar.gz has no ./control: {cnames}")


def verify_apk(data: bytes) -> None:
    if data[:3] != b"ADB":
        raise ValueError(f"apk magic is {data[:4]!r}, expected ADB?/. /d /c")
    mode = data[3:4]
    if mode == b"d":  # deflate level 0, raw stream (no gzip header)
        body = zlib.decompress(data[4:], -zlib.MAX_WBITS)
    elif mode == b".":  # uncompressed
        body = data[4:]
    else:
        raise ValueError(f"unsupported apk v3 compression mode {mode!r}")
    if body[:4] != b"ADB.":
        raise ValueError(f"apk body magic {body[:4]!r}, expected ADB.")
    if body[4:8] != b"pckg":
        raise ValueError(f"apk schema {body[4:8]!r}, expected pckg")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    root = sys.argv[1]
    files = sorted(
        f for f in os.listdir(root) if f.endswith((".ipk", ".apk"))
    )
    if not files:
        print(f"no .ipk/.apk files under {root}")
        return 1
    bad = 0
    for f in files:
        data = open(os.path.join(root, f), "rb").read()
        try:
            if f.endswith(".ipk"):
                verify_ipk(data)
            else:
                verify_apk(data)
        except Exception as e:  # noqa: BLE001 - report every failure
            print(f"INVALID {f}: {e}")
            bad += 1
    print(f"checked {len(files)} packages: {len(files) - bad} valid, {bad} invalid")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

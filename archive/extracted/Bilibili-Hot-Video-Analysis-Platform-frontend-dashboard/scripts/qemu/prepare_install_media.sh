#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${HOME}/qemu"
ISO="${BASE_DIR}/CentOS-7-x86_64-Minimal-2009.iso"
KS_SRC="${BASE_DIR}/ks/centos7.ks"
KS_DIR="${BASE_DIR}/ks_iso"
MNT="${BASE_DIR}/iso_mnt"

mkdir -p "${BASE_DIR}/ks" "${KS_DIR}" "${MNT}"
cp "${KS_SRC}" "${KS_DIR}/centos7.ks"
genisoimage -quiet -o "${BASE_DIR}/ks.iso" -R -J "${KS_DIR}"

isoinfo -R -i "${ISO}" -x /images/pxeboot/vmlinuz > /tmp/vmlinuz
isoinfo -R -i "${ISO}" -x /images/pxeboot/initrd.img > /tmp/initrd.img

echo "prepared ks.iso, /tmp/vmlinuz, /tmp/initrd.img"

#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${HOME}/qemu"
ISO="${BASE_DIR}/CentOS-7-x86_64-Minimal-2009.iso"
KS_ISO="${BASE_DIR}/ks.iso"
DISK="${BASE_DIR}/vms/base.qcow2"
LOG="${BASE_DIR}/install_base.log"

if [[ ! -f "${ISO}" ]]; then
  echo "missing ISO: ${ISO}" >&2
  exit 1
fi

if [[ ! -f /tmp/vmlinuz || ! -f /tmp/initrd.img ]]; then
  echo "missing /tmp/vmlinuz or /tmp/initrd.img; run prepare_install_media.sh first" >&2
  exit 1
fi

mkdir -p "${BASE_DIR}/vms"
rm -f "${DISK}" "${LOG}"
qemu-img create -f qcow2 "${DISK}" 20G

echo "installing CentOS 7 base image"
echo "log: ${LOG}"

qemu-system-x86_64 \
  -enable-kvm \
  -machine accel=kvm \
  -cpu host \
  -m 2048 \
  -smp 2 \
  -audio none \
  -display none \
  -serial file:"${LOG}" \
  -no-reboot \
  -drive file="${DISK}",format=qcow2,if=virtio,cache=writeback,discard=unmap \
  -drive file="${ISO}",media=cdrom,readonly=on,index=0 \
  -drive file="${KS_ISO}",media=cdrom,readonly=on,index=1 \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -kernel /tmp/vmlinuz \
  -initrd /tmp/initrd.img \
  -append "inst.ks=cdrom:/centos7.ks inst.text console=ttyS0,115200n8"

echo "base install qemu exited"

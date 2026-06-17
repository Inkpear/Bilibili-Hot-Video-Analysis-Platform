#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${HOME}/qemu"
ISO="${BASE_DIR}/CentOS-7-x86_64-Minimal-2009.iso"
DISK="${BASE_DIR}/vms/centos7-manual.qcow2"
LOG="${BASE_DIR}/manual-install.log"

mkdir -p "${BASE_DIR}/vms"

if [[ ! -f "${ISO}" ]]; then
  echo "missing ISO: ${ISO}" >&2
  exit 1
fi

if [[ ! -f "${DISK}" ]]; then
  qemu-img create -f qcow2 "${DISK}" 20G
fi

echo "VNC: 127.0.0.1:5901"
echo "Monitor: telnet 127.0.0.1 5555"
echo "Disk: ${DISK}"
echo "Log: ${LOG}"

exec qemu-system-x86_64 \
  -enable-kvm \
  -machine accel=kvm \
  -cpu host \
  -m 2048 \
  -smp 2 \
  -audio none \
  -object secret,id=vncpass,data=hadoop12 \
  -drive file="${DISK}",format=qcow2,if=virtio,cache=writeback,discard=unmap \
  -cdrom "${ISO}" \
  -boot d \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -vnc 127.0.0.1:1,password-secret=vncpass \
  -monitor telnet:127.0.0.1:5555,server,nowait \
  -serial file:"${LOG}"

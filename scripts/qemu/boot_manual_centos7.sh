#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${HOME}/qemu"
DISK="${BASE_DIR}/vms/centos7-manual.qcow2"
LOG="${BASE_DIR}/manual-boot.log"

if [[ ! -f "${DISK}" ]]; then
  echo "missing disk: ${DISK}" >&2
  exit 1
fi

echo "VNC: 127.0.0.1:5901"
echo "SSH: ssh -p 2222 root@127.0.0.1"
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
  -boot c \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -vnc 127.0.0.1:1,password-secret=vncpass \
  -monitor telnet:127.0.0.1:5555,server,nowait \
  -serial file:"${LOG}"

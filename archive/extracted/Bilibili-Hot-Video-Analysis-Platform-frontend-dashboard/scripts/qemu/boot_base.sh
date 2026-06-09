#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${HOME}/qemu"
DISK="${BASE_DIR}/vms/base.qcow2"
LOG="${BASE_DIR}/boot_base.log"

if [[ ! -f "${DISK}" ]]; then
  echo "missing disk: ${DISK}" >&2
  exit 1
fi

rm -f "${LOG}"
exec qemu-system-x86_64 \
  -enable-kvm \
  -machine accel=kvm \
  -cpu host \
  -m 2048 \
  -smp 2 \
  -audio none \
  -display none \
  -serial file:"${LOG}" \
  -drive file="${DISK}",format=qcow2,if=virtio,cache=writeback,discard=unmap \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:2222-:22 \
  -device virtio-net-pci,netdev=net0

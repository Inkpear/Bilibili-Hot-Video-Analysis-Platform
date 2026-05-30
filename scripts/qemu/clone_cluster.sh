#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${HOME}/qemu"
BASE="${BASE_DIR}/vms/base.qcow2"

if [[ ! -f "${BASE}" ]]; then
  echo "missing base image: ${BASE}" >&2
  exit 1
fi

for name in master worker1 worker2; do
  rm -f "${BASE_DIR}/vms/${name}.qcow2"
  qemu-img create -f qcow2 -F qcow2 -b "${BASE}" "${BASE_DIR}/vms/${name}.qcow2"
done

echo "created master.qcow2 worker1.qcow2 worker2.qcow2"

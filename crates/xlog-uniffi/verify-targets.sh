#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: ./verify-targets.sh [--debug|--release|--all-variants] [--host]

Default: --debug for all Android ABIs found in build.gradle.kts (fallback: arm64-v8a, armeabi-v7a).
--host: also run `cargo build -p mars-xlog-uniffi` for the host toolchain.
EOF
}

variant_mode="debug"
build_host="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      variant_mode="debug"
      shift
      ;;
    --release)
      variant_mode="release"
      shift
      ;;
    --all-variants)
      variant_mode="all"
      shift
      ;;
    --host)
      build_host="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ ! -x "${root_dir}/gradlew" ]]; then
  echo "gradlew not found or not executable: ${root_dir}/gradlew" >&2
  exit 1
fi

# Android SDK is required for the Android targets.
if [[ -z "${ANDROID_HOME:-}" && -z "${ANDROID_SDK_ROOT:-}" ]]; then
  echo "ANDROID_HOME or ANDROID_SDK_ROOT is not set. Android targets will fail to build." >&2
  echo "Set the SDK path, or add local.properties with sdk.dir." >&2
  exit 1
fi

abi_filters=()
if [[ -f "${root_dir}/build.gradle.kts" ]]; then
  while IFS= read -r abi; do
    [[ -n "${abi}" ]] && abi_filters+=("${abi}")
  done < <(rg -n "abiFilters" "${root_dir}/build.gradle.kts" | grep -o '"[^"]\\+"' | tr -d '"')
fi

if [[ "${#abi_filters[@]}" -eq 0 ]]; then
  abi_filters=("arm64-v8a" "armeabi-v7a")
fi

map_abi_to_target() {
  case "$1" in
    arm64-v8a) echo "AndroidArm64" ;;
    armeabi-v7a) echo "AndroidArmV7" ;;
    x86_64) echo "AndroidX64" ;;
    x86) echo "AndroidX86" ;;
    *)
      echo ""
      ;;
  esac
}

variants=()
case "${variant_mode}" in
  debug) variants=("Debug") ;;
  release) variants=("Release") ;;
  all) variants=("Debug" "Release") ;;
  *)
    echo "Invalid variant mode: ${variant_mode}" >&2
    exit 2
    ;;
esac

echo "Verifying Android targets with variants: ${variants[*]}"

for abi in "${abi_filters[@]}"; do
  target="$(map_abi_to_target "${abi}")"
  if [[ -z "${target}" ]]; then
    echo "Skipping unknown ABI: ${abi}" >&2
    continue
  fi
  for variant in "${variants[@]}"; do
    task="cargoBuild${target}${variant}"
    echo "==> ./gradlew ${task}"
    (cd "${root_dir}" && ./gradlew "${task}")
  done
done

if [[ "${build_host}" == "true" ]]; then
  echo "==> cargo build -p mars-xlog-uniffi"
  (cd "${root_dir}/../.." && cargo build -p mars-xlog-uniffi)
fi

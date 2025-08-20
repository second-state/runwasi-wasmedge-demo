#!/usr/bin/env bash
set -euo pipefail

# find plugin file dynamically from `wasmedge -v`
PLUGIN_FILE_RAW=$(wasmedge -v 2>&1 | grep "libwasmedgePluginWasiNN.so" | awk '{print $1}')
PLUGIN_FILE=$(realpath "$PLUGIN_FILE_RAW")

PLUGIN_DIR=$(dirname "$PLUGIN_FILE")
LIB_DIR=$(realpath "$PLUGIN_DIR/../lib")

cat > values.yaml <<EOF
# path values to be used by helm
paths:
  wasi_nn_plugin_lib_dir: "$LIB_DIR"
  wasi_nn_plugin_file_dir: "$PLUGIN_DIR"
  wasi_nn_plugin_file: "$PLUGIN_FILE"
  
# systemLibs values to be used by helm
systemLibs:
EOF

# collect system libs with ldd
libs=$(ldd "$PLUGIN_FILE" | awk '
  $2 == "=>" && $3 ~ /^\// {print $3}
  $1 ~ /^\// {print $1}
')

for lib in $libs; do
  name=$(basename "$lib" | tr '.-' '_')
  echo "  - name: $name" >> values.yaml
  echo "    hostPath: $lib" >> values.yaml
done

echo "values.yaml generated ($(echo "$libs" | wc -l) system libs)"

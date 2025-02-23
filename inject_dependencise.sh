#!/bin/bash

if [ $# -ne 2 ]; then
    echo "Usage: $0 <target.so> <destination>"
    exit 1
fi

lib="$1"
destination="$2"

dependencies=$(ldd "$lib" | awk '!/not found/ {print $3}' | grep -v '^$')
missing_dependencies=$(ldd "$lib" | awk '/not found/ {print $1}')

if [ -n "$missing_dependencies" ]; then
    echo "Missing dependencies:"
    echo "$missing_dependencies"
    echo "Please install the missing dependencies on your host before injecting them used for shim."
    exit 1
else
    sudo cp "$lib" "$destination"
    for dep in $dependencies; do
        if [ -f "$dep" ]; then
            sudo cp "$dep" "$destination"
        fi
    done
fi

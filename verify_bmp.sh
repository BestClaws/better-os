#!/bin/bash

# Script to verify if all sprite files match their references

SPRITES_DIR="sprites"
REFERENCE_DIR="reference_sprites"

# Check if directories exist
if [ ! -d "$SPRITES_DIR" ]; then
    echo "FAILED:"
    echo "Error: $SPRITES_DIR directory not found" >&2
    exit 1
fi

if [ ! -d "$REFERENCE_DIR" ]; then
    echo "FAILED:"
    echo "Error: $REFERENCE_DIR directory not found" >&2
    exit 1
fi

# Array to store failed files
failed_files=()

# Check all sprite files
for sprite_file in "$SPRITES_DIR"/*.bmp; do
    if [ ! -f "$sprite_file" ]; then
        continue
    fi
    
    filename=$(basename "$sprite_file")
    reference_file="$REFERENCE_DIR/$filename"
    
    # Check if reference file exists
    if [ ! -f "$reference_file" ]; then
        failed_files+=("$filename (reference missing)")
        continue
    fi
    
    # Calculate checksums
    sprite_checksum=$(sha256sum "$sprite_file" | awk '{print $1}')
    reference_checksum=$(sha256sum "$reference_file" | awk '{print $1}')
    
    # Compare checksums
    if [ "$sprite_checksum" != "$reference_checksum" ]; then
        failed_files+=("$filename")
    fi
done

# Output results
if [ ${#failed_files[@]} -eq 0 ]; then
    echo "OK"
    exit 0
else
    echo "FAILED:"
    for file in "${failed_files[@]}"; do
        echo "$file"
    done
    exit 1
fi

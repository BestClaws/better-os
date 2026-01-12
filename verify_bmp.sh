#!/bin/bash

# Script to verify if all sprite files match their references

SPRITES_DIR="rust-gfx/sprites"
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

# Arrays to store results
matched_files=()
failed_files=()

# Check all reference files (only compare sprites that have references)
for reference_file in "$REFERENCE_DIR"/*.bmp; do
    if [ ! -f "$reference_file" ]; then
        continue
    fi
    
    filename=$(basename "$reference_file")
    sprite_file="$SPRITES_DIR/$filename"
    
    # Check if sprite file exists
    if [ ! -f "$sprite_file" ]; then
        failed_files+=("$filename (sprite missing)")
        continue
    fi
    
    # Calculate checksums
    sprite_checksum=$(sha256sum "$sprite_file" | awk '{print $1}')
    reference_checksum=$(sha256sum "$reference_file" | awk '{print $1}')
    
    # Compare checksums
    if [ "$sprite_checksum" != "$reference_checksum" ]; then
        failed_files+=("$filename")
    else
        matched_files+=("$filename")
    fi
done

# Output results
echo "MATCHED (${#matched_files[@]}):"
for file in "${matched_files[@]}"; do
    echo "$file"
done

echo ""
echo "FAILED (${#failed_files[@]}):"
for file in "${failed_files[@]}"; do
    echo "$file"
done

echo ""
echo "Summary: ${#matched_files[@]} matched, ${#failed_files[@]} failed out of $((${#matched_files[@]} + ${#failed_files[@]})) total"

if [ ${#failed_files[@]} -eq 0 ]; then
    exit 0
else
    exit 1
fi

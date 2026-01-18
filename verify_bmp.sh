#!/bin/bash

# Script to verify if all sprite files match their references

SPRITES_DIR="sprite-generator/sprites"
REFERENCE_DIR="reference_sprites"

# Check for stale sprite directories in legacy locations
if [ -d "sprites" ]; then
    echo "FAILED:"
    echo "Error: sprites/ directory found in project root!" >&2
    echo "This is WRONG! Sprites should now be generated under sprite-generator/sprites/." >&2
    echo "Please delete the incorrect sprites/ directory:" >&2
    echo "  rm -rf sprites" >&2
    echo "Then regenerate sprites from the correct location:" >&2
    echo "  cd sprite-generator && cargo run --release --target x86_64-unknown-linux-gnu --bin sprite-generator" >&2
    exit 1
fi

if [ -d "rust-gfx/sprites" ]; then
    echo "FAILED:"
    echo "Error: legacy rust-gfx/sprites directory detected." >&2
    echo "Please delete it to avoid confusing the verification script:" >&2
    echo "  rm -rf rust-gfx/sprites" >&2
    echo "Then regenerate sprites from the new crate:" >&2
    echo "  cd sprite-generator && cargo run --release --target x86_64-unknown-linux-gnu --bin sprite-generator" >&2
    exit 1
fi

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

# Check if sprites directory has any BMP files
sprite_count=$(find "$SPRITES_DIR" -maxdepth 1 -name "*.bmp" 2>/dev/null | wc -l)
if [ "$sprite_count" -eq 0 ]; then
    echo "FAILED:"
    echo "Error: No sprite files found in $SPRITES_DIR" >&2
    echo "Please run sprite generator first:" >&2
    echo "  cd sprite-generator" >&2
    echo "  cargo run --release --target x86_64-unknown-linux-gnu --bin sprite-generator" >&2
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

# Clean up sprites after verification
echo ""
echo "Cleaning up sprites directory..."
rm -f "$SPRITES_DIR"/*.bmp
echo "Sprites directory cleared."

if [ ${#failed_files[@]} -eq 0 ]; then
    exit 0
else
    exit 1
fi

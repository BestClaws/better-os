#!/usr/bin/env python3
"""
Sprite verification utility.

Usage:
    ./verify_sprites.py calculate  - Calculate SHA256 hashes and store in output/checksums.txt
    ./verify_sprites.py verify     - Verify current files against stored checksums
"""

import hashlib
import sys
from pathlib import Path


def calculate_sha256(filepath):
    """Calculate SHA256 hash of a file."""
    sha256 = hashlib.sha256()
    with open(filepath, 'rb') as f:
        while chunk := f.read(8192):
            sha256.update(chunk)
    return sha256.hexdigest()


def calculate_checksums(output_dir):
    """Calculate checksums for all .bmp files and store in checksums.txt."""
    output_path = Path(output_dir)
    checksums_file = output_path / "checksums.txt"
    
    # Find all .bmp files
    bmp_files = sorted(output_path.glob("*.bmp"))
    
    if not bmp_files:
        print(f"No .bmp files found in {output_dir}")
        return
    
    print(f"Calculating checksums for {len(bmp_files)} files...")
    
    with open(checksums_file, 'w') as f:
        for bmp_file in bmp_files:
            sha256 = calculate_sha256(bmp_file)
            f.write(f"{sha256}  {bmp_file.name}\n")
            print(f"  {bmp_file.name}: {sha256}")
    
    print(f"\nChecksums saved to {checksums_file}")


def verify_checksums(output_dir):
    """Verify current files against stored checksums."""
    output_path = Path(output_dir)
    checksums_file = output_path / "checksums.txt"
    
    if not checksums_file.exists():
        print(f"Error: {checksums_file} not found. Run 'calculate' first.")
        sys.exit(1)
    
    # Read stored checksums
    stored_checksums = {}
    with open(checksums_file, 'r') as f:
        for line in f:
            line = line.strip()
            if line:
                sha256, filename = line.split(None, 1)
                stored_checksums[filename] = sha256
    
    print(f"Verifying {len(stored_checksums)} files...\n")
    
    matches = 0
    mismatches = 0
    missing = 0
    
    for filename, expected_sha in stored_checksums.items():
        filepath = output_path / filename
        
        if not filepath.exists():
            print(f"✗ MISSING: {filename}")
            missing += 1
            continue
        
        actual_sha = calculate_sha256(filepath)
        
        if actual_sha == expected_sha:
            print(f"✓ MATCH: {filename}")
            matches += 1
        else:
            print(f"✗ MISMATCH: {filename}")
            print(f"  Expected: {expected_sha}")
            print(f"  Actual:   {actual_sha}")
            mismatches += 1
    
    # Check for extra files not in checksums
    all_bmp_files = set(f.name for f in output_path.glob("*.bmp"))
    extra_files = all_bmp_files - set(stored_checksums.keys())
    
    if extra_files:
        print(f"\n{len(extra_files)} extra files not in checksums:")
        for filename in sorted(extra_files):
            print(f"  + {filename}")
    
    # Summary
    print(f"\n{'='*60}")
    print(f"Summary:")
    print(f"  Matches:    {matches}")
    print(f"  Mismatches: {mismatches}")
    print(f"  Missing:    {missing}")
    if extra_files:
        print(f"  Extra:      {len(extra_files)}")
    print(f"{'='*60}")
    
    if mismatches > 0 or missing > 0:
        sys.exit(1)
    else:
        print("All files verified successfully!")


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ['calculate', 'verify']:
        print(__doc__)
        sys.exit(1)
    
    command = sys.argv[1]
    output_dir = Path(__file__).parent / "output"
    
    if not output_dir.exists():
        print(f"Error: {output_dir} directory not found")
        sys.exit(1)
    
    if command == 'calculate':
        calculate_checksums(output_dir)
    elif command == 'verify':
        verify_checksums(output_dir)


if __name__ == '__main__':
    main()

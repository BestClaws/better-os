#!/usr/bin/env python3
"""
Compare two BMP files pixel-by-pixel and show differences
"""
import sys
import struct

def read_bmp(filename):
    with open(filename, 'rb') as f:
        # Read BMP header
        header = f.read(54)
        if header[0:2] != b'BM':
            raise ValueError("Not a BMP file")
        
        width = struct.unpack('<i', header[18:22])[0]
        height_raw = struct.unpack('<i', header[22:26])[0]
        
        # Height can be negative (top-down) or positive (bottom-up)
        if height_raw < 0:
            height = -height_raw
            top_down = True
        else:
            height = height_raw
            top_down = False
        
        # Read pixel data
        f.seek(54)
        pixels = []
        for y in range(height):
            row = []
            for x in range(width):
                b, g, r, a = struct.unpack('BBBB', f.read(4))
                row.append((r, g, b, a))
            pixels.append(row)
        
        # If bottom-up (default BMP), flip to top-down for easier comparison
        if not top_down:
            pixels = list(reversed(pixels))
        
        return width, height, pixels

def compare_bmps(ref_file, test_file):
    w1, h1, pix1 = read_bmp(ref_file)
    w2, h2, pix2 = read_bmp(test_file)
    
    if w1 != w2 or h1 != h2:
        print(f"Size mismatch: {w1}x{h1} vs {w2}x{h2}")
        return
    
    diff_count = 0
    first_diffs = []
    
    for y in range(h1):
        for x in range(w1):
            p1 = pix1[y][x]
            p2 = pix2[y][x]
            if p1 != p2:
                diff_count += 1
                if len(first_diffs) < 20:
                    first_diffs.append((x, y, p1, p2))
    
    print(f"Total pixels: {w1 * h1}")
    print(f"Different pixels: {diff_count}")
    print(f"Match rate: {100 * (1 - diff_count / (w1 * h1)):.2f}%")
    print(f"\nFirst {len(first_diffs)} differences (top-down coordinates):")
    for x, y, p1, p2 in first_diffs:
        print(f"  ({x},{y}): LVGL={p1} Rust={p2}")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: compare_bmp.py <reference.bmp> <test.bmp>")
        sys.exit(1)
    
    compare_bmps(sys.argv[1], sys.argv[2])

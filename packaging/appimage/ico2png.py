#!/usr/bin/env python3
"""ico2png: extract one BMP/DIB entry of an .ico and write it as PNG (stdlib only).

Usage: python3 -I ico2png.py ICO PNG [SIZE]
  ICO   input .ico file (BMP entries, 32bpp)
  PNG   output PNG path
  SIZE  wanted square size, default 256
Exit code: 0 ok, 1 on any error (message on stderr).
"""
import struct
import sys
import zlib


def read_entry(data: bytes, size: int) -> tuple[int, int, bytes]:
    """Find entry of given size. Args: data = ico bytes, size = pixels. Returns (w, h, rgba)."""
    _, kind, count = struct.unpack_from("<HHH", data, 0)
    if kind != 1:
        raise ValueError("not an ico file")
    for i in range(count):
        w, h, _c, _r, _p, _b, nbytes, off = struct.unpack_from("<BBBBHHII", data, 6 + 16 * i)
        w = w or 256
        h = h or 256
        if w != size or h != size:
            continue
        img = data[off:off + nbytes]
        if img[:8] == b"\x89PNG\r\n\x1a\n":
            raise ValueError("entry is already PNG")
        hdr, bw, bh2, _pl, bpp = struct.unpack_from("<IiiHH", img, 0)
        if bpp != 32:
            raise ValueError(f"unsupported bpp {bpp}")
        # BMP height counts XOR+AND masks; real height = half.
        bh = abs(bh2) // 2
        px = img[hdr:hdr + bw * bh * 4]
        rows = []
        # Bottom-up rows, BGRA -> RGBA.
        for y in range(bh - 1, -1, -1):
            row = px[y * bw * 4:(y + 1) * bw * 4]
            out = bytearray(len(row))
            out[0::4] = row[2::4]
            out[1::4] = row[1::4]
            out[2::4] = row[0::4]
            out[3::4] = row[3::4]
            rows.append(bytes(out))
        return bw, bh, b"".join(b"\x00" + r for r in rows)
    raise ValueError(f"no {size}x{size} entry")


def chunk(tag: bytes, body: bytes) -> bytes:
    """Build PNG chunk. Args: tag, body. Returns bytes."""
    return (struct.pack(">I", len(body)) + tag + body
            + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF))


def main() -> int:
    """CLI entry. Returns exit code."""
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 1
    size = int(sys.argv[3]) if len(sys.argv) > 3 else 256
    try:
        with open(sys.argv[1], "rb") as f:
            w, h, raw = read_entry(f.read(), size)
    except (OSError, ValueError, struct.error) as e:
        print(f"ico2png: {e}", file=sys.stderr)
        return 1
    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw, 9))
           + chunk(b"IEND", b""))
    with open(sys.argv[2], "wb") as f:
        f.write(png)
    return 0


if __name__ == "__main__":
    sys.exit(main())

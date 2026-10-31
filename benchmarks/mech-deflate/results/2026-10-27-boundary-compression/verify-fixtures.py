import ctypes as c
import hashlib
import json
from pathlib import Path
import zlib


class Stream(c.Structure):
    _fields_ = [
        ("next_in", c.c_void_p), ("avail_in", c.c_uint), ("total_in", c.c_ulong),
        ("next_out", c.c_void_p), ("avail_out", c.c_uint), ("total_out", c.c_ulong),
        ("msg", c.c_char_p), ("state", c.c_void_p),
        ("zalloc", c.c_void_p), ("zfree", c.c_void_p), ("opaque", c.c_void_p),
        ("data_type", c.c_int), ("adler", c.c_ulong), ("reserved", c.c_ulong),
    ]


def blocks(raw, expected):
    library = c.CDLL("/usr/lib/libz.dylib")
    library.zlibVersion.restype = c.c_char_p
    library.inflateInit2_.argtypes = [c.POINTER(Stream), c.c_int, c.c_char_p, c.c_int]
    library.inflate.argtypes = [c.POINTER(Stream), c.c_int]
    library.inflateEnd.argtypes = [c.POINTER(Stream)]
    source = c.create_string_buffer(raw)
    destination = c.create_string_buffer(len(expected) + 1)
    stream = Stream()
    stream.next_in = c.addressof(source)
    stream.avail_in = len(raw)
    stream.next_out = c.addressof(destination)
    stream.avail_out = len(destination)
    assert library.inflateInit2_(c.byref(stream), -15, library.zlibVersion(), c.sizeof(Stream)) == 0
    result = []
    begin = 0
    produced = 0
    try:
        while True:
            before = (stream.total_in, stream.total_out)
            code = library.inflate(c.byref(stream), 5)  # Z_BLOCK
            assert code in (0, 1), (code, stream.msg)
            if stream.data_type & 128:
                end = stream.total_in * 8 - (stream.data_type & 63)
                if end > begin:
                    header = int.from_bytes(raw[begin // 8:begin // 8 + 2], "little") >> (begin % 8)
                    result.append({
                        "begin_bit": begin, "end_bit": end,
                        "kind": (header >> 1) & 3, "final": header & 1,
                        "input_begin": produced, "input_end": stream.total_out,
                    })
                    begin = end
                    produced = stream.total_out
            if code == 1:
                break
            assert (stream.total_in, stream.total_out) != before, "inflate made no progress"
        assert destination.raw[:stream.total_out] == expected
        assert stream.total_out == len(expected)
        assert result[-1]["final"] == 1
    finally:
        assert library.inflateEnd(c.byref(stream)) == 0
    return result


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[4]
    fixtures = root / "hosts/gpu/tests/fixtures/native"
    manifest = json.loads((fixtures / "deflate-boundary-reference.json").read_text())
    for item in manifest["cases"]:
        source = (fixtures / item["input_fixture"]).read_bytes()
        raw = (fixtures / item["deflate_fixture"]).read_bytes()
        assert len(source) == item["input_bytes"]
        assert len(raw) == item["raw_bytes"]
        assert hashlib.sha256(source).hexdigest() == item["input_sha256"]
        assert hashlib.sha256(raw).hexdigest() == item["raw_sha256"]
        assert zlib.decompress(raw, -15) == source
        assert blocks(raw, source) == item["blocks"]
        print(json.dumps({"case": item["name"], "input_bytes": len(source), "raw_bytes": len(raw), "blocks": item["blocks"]}))
    assert len(manifest["cases"][0]["blocks"]) == 2
    assert manifest["cases"][1]["input_bytes"] == 32768 + 259

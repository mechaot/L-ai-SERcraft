"""Unit tests for resx2json."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import resx2json as r

HDR = '<root><resheader name="x"><value>y</value></resheader>'


def write(p: Path, body: str) -> None:
    """Write a resx file.

    Args:
        p: target path.
        body: data elements xml.
    Returns:
        None.
    """
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(HDR + body + "</root>", encoding="utf-8")


class T(unittest.TestCase):
    def test_plain_typed_meta(self):
        with tempfile.TemporaryDirectory() as d:
            f = Path(d) / "A.resx"
            write(f, '<data name="b.Text" xml:space="preserve"><value>a\r\nb</value></data>'
                     '<data name="b.Size" type="System.Drawing.Size, System.Drawing"><value>1, 2</value></data>'
                     '<data name="i" type="System.Resources.ResXFileRef"><value>x</value></data>'
                     '<data name="b.bin" mimetype="application/x-microsoft.net.object.binary.base64"><value>QQ==</value></data>'
                     '<data name="&gt;&gt;b.Name"><value>b</value></data>'
                     '<data name="$this.Text"><value>T</value></data>')
            self.assertEqual(r.parse_resx(f), {"b.Text": "a\nb", "$this.Text": "T"})

    def test_culture(self):
        self.assertEqual(r.split_culture("MainForm"), ("MainForm", "en"))
        self.assertEqual(r.split_culture("MainForm.it"), ("MainForm", "it"))
        self.assertEqual(r.split_culture("Strings.zh-CN"), ("Strings", "zh-CN"))

    def test_collect_prefix(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            write(root / "Strings.resx", '<data name="K"><value>v</value></data>')
            write(root / "Strings.de.resx", '<data name="K"><value>w</value></data>')
            write(root / "SvgConverter" / "C.resx", '<data name="x.Text"><value>s</value></data>')
            write(root / "RasterConverter" / "C.fr.resx", '<data name="x.Text"><value>r</value></data>')
            write(root / "SharpGL" / "Z.resx", '<data name="x.Text"><value>no</value></data>')
            res = r.collect(root)
            self.assertEqual(res["en"], {"Strings.K": "v", "SvgConverter.C.x.Text": "s"})
            self.assertEqual(res["de"], {"Strings.K": "w"})
            self.assertEqual(res["fr"], {"RasterConverter.C.x.Text": "r"})

    def test_only_culture_file(self):
        with tempfile.TemporaryDirectory() as d:
            write(Path(d) / "Only.ja-JP.resx", '<data name="a.Text"><value>x</value></data>')
            self.assertEqual(r.collect(Path(d)), {"ja-JP": {"Only.a.Text": "x"}})


if __name__ == "__main__":
    unittest.main()

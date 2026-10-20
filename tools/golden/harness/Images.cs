// Deterministic synthetic bitmaps. Saved as PNG; harness reloads the PNG so the oracle sees exactly what Rust will decode.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;

namespace Golden
{
	public static class Images
	{
		public delegate Color Px(int x, int y);

		// y = 0 is the TOP row of the saved PNG (normal image convention).
		public static Bitmap Make(int w, int h, Px f)
		{
			Bitmap b = new Bitmap(w, h, PixelFormat.Format32bppArgb);
			for (int y = 0; y < h; y++)
				for (int x = 0; x < w; x++)
					b.SetPixel(x, y, f(x, y));
			return b;
		}

		static Color G(int v) { return Color.FromArgb(255, v, v, v); }
		static readonly Color Black = Color.FromArgb(255, 0, 0, 0);
		static readonly Color White = Color.FromArgb(255, 255, 255, 255);

		// simple LCG so noise is identical everywhere
		static uint seed;
		static int Rnd(int n) { seed = seed * 1664525u + 1013904223u; return (int)((seed >> 8) % (uint)n); }

		// name -> factory. Order fixed (alphabetical by name when iterated through Names()).
		static Dictionary<string, Func<Bitmap>> grayscale = new Dictionary<string, Func<Bitmap>>();
		static Dictionary<string, Func<Bitmap>> bw = new Dictionary<string, Func<Bitmap>>();

		static Images()
		{
			grayscale["black_8x8"] = () => Make(8, 8, (x, y) => Black);
			grayscale["white_8x8"] = () => Make(8, 8, (x, y) => White);
			grayscale["single_1x1"] = () => Make(1, 1, (x, y) => G(40));
			grayscale["gray_ramp_16x4"] = () => Make(16, 4, (x, y) => G(x * 17));
			grayscale["gray_ramp_v_4x16"] = () => Make(4, 16, (x, y) => G(y * 17));
			grayscale["checker_6x6"] = () => Make(6, 6, (x, y) => ((x + y) % 2 == 0) ? Black : White);
			grayscale["col_1x20"] = () => Make(1, 20, (x, y) => G((y * 37) % 256));
			grayscale["row_20x1"] = () => Make(20, 1, (x, y) => G((x * 53) % 256));
			grayscale["single_px_9x9"] = () => Make(9, 9, (x, y) => (x == 4 && y == 4) ? Black : White);
			grayscale["alpha_ramp_12x3"] = () => Make(12, 3, (x, y) => Color.FromArgb(x * 23, y == 0 ? 0 : (y == 1 ? 100 : 255), 0, 0));
			grayscale["lines_10x10"] = () => Make(10, 10, (x, y) => (y == 3 || x == 6) ? Black : White);
			grayscale["diag_12x12"] = () => Make(12, 12, (x, y) => (x == y) ? Black : ((x + y == 11) ? G(128) : White));
			grayscale["rect_20x12"] = () => Make(20, 12, (x, y) => (x >= 4 && x < 16 && y >= 3 && y < 9) ? Black : ((x >= 2 && x < 18 && y >= 1 && y < 11) ? G(200) : White));
			grayscale["circle_24x24"] = () => Make(24, 24, (x, y) =>
			{
				double dx = x - 11.5, dy = y - 11.5, d = Math.Sqrt(dx * dx + dy * dy);
				int v = d < 8 ? 0 : (d < 10 ? (int)((d - 8) * 127.5) : 255);
				return G(v);
			});
			grayscale["noise_16x16"] = () => { seed = 12345; return Make(16, 16, (x, y) => { int r = Rnd(4); return r == 0 ? White : G(Rnd(256)); }); };
			grayscale["stripes_32x5"] = () => Make(32, 5, (x, y) => ((x / (y + 1)) % 2 == 0) ? Black : G(60 * (y % 3)));
			grayscale["rgb_mix_8x8"] = () => Make(8, 8, (x, y) => Color.FromArgb(255, (x * 36) % 256, (y * 36) % 256, 255 - x * 30));
			grayscale["alpha_white_6x2"] = () => Make(6, 2, (x, y) => Color.FromArgb(y == 0 ? 255 : x * 50, 255, 255, 255));

			// black/white shapes for potrace (black = shape)
			bw["square_16x16"] = () => Make(16, 16, (x, y) => (x >= 3 && x < 13 && y >= 3 && y < 13) ? Black : White);
			bw["ring_24x24"] = () => Make(24, 24, (x, y) => { double dx = x - 11.5, dy = y - 11.5, d = Math.Sqrt(dx * dx + dy * dy); return (d < 10 && d > 5) ? Black : White; });
			bw["disc_20x20"] = () => Make(20, 20, (x, y) => { double dx = x - 9.5, dy = y - 9.5; return (dx * dx + dy * dy < 64) ? Black : White; });
			bw["triangle_24x20"] = () => Make(24, 20, (x, y) => (y >= 2 && y < 18 && x >= 2 && x <= 2 + (y - 2) * 20 / 16) ? Black : White);
			bw["two_blobs_28x14"] = () => Make(28, 14, (x, y) => { double a = (x - 7) * (x - 7) + (y - 7) * (y - 7); double b = (x - 20) * (x - 20) / 2.0 + (y - 7) * (y - 7); return (a < 20 || b < 22) ? Black : White; });
			bw["letter_E_16x20"] = () => Make(16, 20, (x, y) => ((x >= 3 && x < 6 && y >= 2 && y < 18) || (x >= 3 && x < 13 && (y < 5 || (y >= 9 && y < 12) || y >= 15) && y >= 2 && y < 18)) ? Black : White);
			bw["letter_O_hole_20x20"] = () => Make(20, 20, (x, y) => { double dx = (x - 9.5) / 8.0, dy = (y - 9.5) / 8.0; double e = dx * dx + dy * dy; double i = (dx * dx) / 0.25 + (dy * dy) / 0.4; return (e < 1 && i > 1) ? Black : White; });
			bw["noise_blobs_24x24"] = () => { seed = 777; return Make(24, 24, (x, y) => (Rnd(100) < 38) ? Black : White); };
			bw["nested_squares_24x24"] = () => Make(24, 24, (x, y) => { int d = Math.Min(Math.Min(x, y), Math.Min(23 - x, 23 - y)); return (d % 4 == 1) ? Black : White; });
			bw["thin_lines_20x20"] = () => Make(20, 20, (x, y) => (x == 3 || y == 10 || x == y) ? Black : White);
			bw["comb_24x16"] = () => Make(24, 16, (x, y) => ((y >= 2 && y < 5 && x >= 2 && x < 22) || (y >= 5 && y < 14 && x % 4 < 2 && x >= 2 && x < 22)) ? Black : White);
			bw["cross_21x21"] = () => Make(21, 21, (x, y) => (Math.Abs(x - 10) <= 2 || Math.Abs(y - 10) <= 2) ? Black : White);
		}

		public static List<string> GrayNames() { List<string> l = new List<string>(grayscale.Keys); l.Sort(StringComparer.Ordinal); return l; }
		public static List<string> BwNames() { List<string> l = new List<string>(bw.Keys); l.Sort(StringComparer.Ordinal); return l; }

		// Save <name>.png into dir (once) and return a fresh Bitmap decoded from that file.
		// Verifies decode == generated pixels.
		public static Bitmap Reload(string dir, string name)
		{
			Func<Bitmap> f = grayscale.ContainsKey(name) ? grayscale[name] : bw[name];
			string path = Path.Combine(dir, name + ".png");
			using (Bitmap gen = f())
			{
				if (!File.Exists(path))
				{
					Directory.CreateDirectory(dir);
					gen.Save(path, ImageFormat.Png);
				}
				Bitmap back = new Bitmap(path);
				if (back.Width != gen.Width || back.Height != gen.Height) throw new Exception("png size mismatch " + name);
				for (int y = 0; y < gen.Height; y++)
					for (int x = 0; x < gen.Width; x++)
					{
						Color a = gen.GetPixel(x, y), b = back.GetPixel(x, y);
						if (a.ToArgb() != b.ToArgb()) throw new Exception(string.Format("png roundtrip mismatch {0} at {1},{2}: {3} vs {4}", name, x, y, a, b));
					}
				return back;
			}
		}
	}
}

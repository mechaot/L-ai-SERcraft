// Golden generator for lg-raster. Calls the original LaserGRBL ImageTransform (Mono + libgdiplus).
// Usage: mono GenRaster.exe <outRoot>   (writes <outRoot>/input, <outRoot>/output, <outRoot>/manifest.txt)
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text;
using LaserGRBL.RasterConverter;

static class GenRaster
{
	static string root, inDir, outDir;
	static StringBuilder manifest = new StringBuilder();
	static CultureInfo inv = CultureInfo.InvariantCulture;
	static Assembly asm = typeof(ImageTransform).Assembly;

	static string F(float f) { return f.ToString("R", inv); }

	// Make a 32bppArgb copy so every op sees the same format as ImageProcessor.
	static Bitmap Load(string name)
	{
		using (Bitmap raw = new Bitmap(Path.Combine(inDir, name + ".png")))
			return raw.Clone(new Rectangle(0, 0, raw.Width, raw.Height), PixelFormat.Format32bppArgb);
	}

	static void Save(Bitmap bmp, string outName)
	{
		bmp.Save(Path.Combine(outDir, outName + ".png"), ImageFormat.Png);
	}

	static void Rec(string outName, string op, string input, params string[] args)
	{
		manifest.Append(outName + " " + op + " " + input);
		foreach (string a in args) manifest.Append(" " + a);
		manifest.AppendLine();
	}

	static Bitmap Make(int w, int h, Func<int, int, Color> f)
	{
		Bitmap b = new Bitmap(w, h, PixelFormat.Format32bppArgb);
		for (int y = 0; y < h; y++)
			for (int x = 0; x < w; x++)
				b.SetPixel(x, y, f(x, y));
		return b;
	}

	static void MakeInputs()
	{
		Random rnd = new Random(42);
		int W = 37, H = 23;
		Make(W, H, (x, y) => Color.FromArgb(255, x * 255 / (W - 1), y * 255 / (H - 1), (x * 7 + y * 11) % 256)).Save(Path.Combine(inDir, "grad_rgb.png"), ImageFormat.Png);
		Make(W, H, (x, y) => Color.FromArgb(255, rnd.Next(256), rnd.Next(256), rnd.Next(256))).Save(Path.Combine(inDir, "noise.png"), ImageFormat.Png);
		Make(W, H, (x, y) => { int v = (x * 255 / (W - 1) + y * 3) % 256; return Color.FromArgb(255, v, v, v); }).Save(Path.Combine(inDir, "gray_ramp.png"), ImageFormat.Png);
		Make(W, H, (x, y) => Color.FromArgb(x * 255 / (W - 1), (x * 13 + 40) % 256, (y * 20 + 90) % 256, 200 - y * 5)).Save(Path.Combine(inDir, "alpha_ramp.png"), ImageFormat.Png);
		Make(W, H, (x, y) =>
		{
			int r = 255 - ((x * 7) % 40), g = 255 - ((y * 5) % 40), b = 255 - ((x + y) % 50);
			int a = (x % 11 == 0) ? 0 : ((y % 7 == 0) ? 128 : 255);
			return Color.FromArgb(a, r, g, b);
		}).Save(Path.Combine(inDir, "nearwhite.png"), ImageFormat.Png);
		// blocks: white bg, black frame, dark gray box, light gray disc, noisy-ish area
		Make(W, H, (x, y) =>
		{
			if (x == 0 || y == 0 || x == W - 1 || y == H - 1) return Color.Black;
			if (x >= 5 && x <= 14 && y >= 4 && y <= 12) return Color.FromArgb(255, 40, 40, 40);
			int dx = x - 26, dy = y - 12;
			if (dx * dx + dy * dy <= 30) return Color.FromArgb(255, 180, 190, 200);
			if (y > 16 && x > 4 && x < 20) return Color.FromArgb(255, 200 + (x % 3) * 20, 220, 230 + (y % 2) * 10);
			return Color.White;
		}).Save(Path.Combine(inDir, "blocks.png"), ImageFormat.Png);
		int BW = 64, BH = 48;
		Make(BW, BH, (x, y) => Color.FromArgb(255, x * 255 / (BW - 1), y * 255 / (BH - 1), ((x / 4 + y / 4) % 2) * 255)).Save(Path.Combine(inDir, "big_pattern.png"), ImageFormat.Png);
	}

	static readonly string[] Opaque = { "grad_rgb", "noise", "gray_ramp" };
	static readonly string[] AllIn = { "grad_rgb", "noise", "gray_ramp", "alpha_ramp" };

	static void GenGray()
	{
		float[][] ps = new float[][] {
			new float[] {0, 0, 0, 0, 1},
			new float[] {1, 1, 1, 0.2f, 1.0f},
			new float[] {0.5f, 1.5f, 0.8f, 0, 1.3f},
			new float[] {1, 1, 1, -0.25f, 1.1f},
			new float[] {1.2f, 0.4f, 1, 0.1f, 0.7f},
		};
		foreach (string inp in AllIn)
			for (int f = 0; f < 4; f++)
				for (int p = 0; p < ps.Length; p++)
				{
					float[] a = ps[p];
					using (Bitmap src = Load(inp))
					using (Bitmap res = ImageTransform.GrayScale(src, a[0], a[1], a[2], a[3], a[4], (ImageTransform.Formula)f))
					{
						string n = "gs_" + inp + "_f" + f + "_p" + p;
						Save(res, n);
						Rec(n, "gs", inp, f.ToString(), F(a[0]), F(a[1]), F(a[2]), F(a[3]), F(a[4]));
					}
				}
	}

	static void GenWhitenize()
	{
		foreach (int t in new int[] { 0, 10, 30, 128 })
			foreach (bool demo in new bool[] { false, true })
				using (Bitmap src = Load("nearwhite"))
				using (Bitmap res = ImageTransform.Whitenize(src, t, demo))
				{
					string n = "whiten_t" + t + "_" + (demo ? "demo" : "clear");
					Save(res, n);
					Rec(n, "whiten", "nearwhite", t.ToString(), demo ? "1" : "0");
				}
	}

	static void GenThreshold()
	{
		foreach (string inp in AllIn)
			foreach (float t in new float[] { 0f, 0.3f, 0.5f, 0.77f, 1f })
				foreach (bool apply in new bool[] { true, false })
				{
					if (!apply && t != 0.5f) continue;
					using (Bitmap src = Load(inp))
					using (Bitmap res = ImageTransform.Threshold(src, t, apply))
					{
						string n = "thr_" + inp + "_" + F(t).Replace(".", "p") + "_" + (apply ? "1" : "0");
						Save(res, n);
						Rec(n, "thr", inp, F(t), apply ? "1" : "0");
					}
				}
	}

	static void GenInvert()
	{
		foreach (string inp in AllIn)
			using (Bitmap src = Load(inp))
			using (Bitmap res = ImageTransform.InvertingImage(src))
			{
				string n = "inv_" + inp;
				Save(res, n);
				Rec(n, "inv", inp);
			}
	}

	// Random mode: DitherImage seeds with TickCount, so replay the loop with a fixed seed.
	static Bitmap DitherRandomSeeded(Bitmap img, int seed)
	{
		Type util = asm.GetType("Cyotek.DitheringTest.Helpers.ImageUtilities");
		var getPix = util.GetMethod("GetPixelsFrom32BitArgbImage", BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Static);
		var toBmp = util.GetMethod("ToBitmap", BindingFlags.NonPublic | BindingFlags.Public | BindingFlags.Static);
		var transform = typeof(ImageTransform).GetMethod("TransformPixel", BindingFlags.NonPublic | BindingFlags.Static);
		Type rdT = asm.GetType("Cyotek.Drawing.Imaging.ColorReduction.RandomDithering");
		object dither = Activator.CreateInstance(rdT, new object[] { seed });
		Type ied = asm.GetType("Cyotek.Drawing.Imaging.ColorReduction.IErrorDiffusion");
		var diffuse = ied.GetMethod("Diffuse");
		Array data = (Array)getPix.Invoke(null, new object[] { img });
		int w = img.Width, h = img.Height;
		for (int row = 0; row < h; row++)
			for (int col = 0; col < w; col++)
			{
				int index = row * w + col;
				object current = data.GetValue(index);
				object transformed = transform.Invoke(null, new object[] { current });
				data.SetValue(transformed, index);
				diffuse.Invoke(dither, new object[] { data, current, transformed, col, row, w, h });
			}
		return (Bitmap)toBmp.Invoke(null, new object[] { data, new Size(w, h) });
	}

	static void GenDither()
	{
		string[] names = Enum.GetNames(typeof(ImageTransform.DitheringMode));
		foreach (string inp in AllIn)
			for (int m = 0; m < names.Length; m++)
			{
				using (Bitmap src = Load(inp))
				{
					Bitmap res = (m == (int)ImageTransform.DitheringMode.Random)
						? DitherRandomSeeded(src, 12345)
						: ImageTransform.DitherImage(src, (ImageTransform.DitheringMode)m);
					string n = "dith_" + inp + "_m" + m;
					Save(res, n);
					Rec(n, "dither", inp, m.ToString(), "12345");
					res.Dispose();
				}
			}
	}

	static void GenFill()
	{
		MethodInfo fill = typeof(ImageTransform).GetMethod("Fill", BindingFlags.NonPublic | BindingFlags.Static);
		MethodInfo outl = typeof(ImageTransform).GetMethod("Outliner", BindingFlags.NonPublic | BindingFlags.Static);
		int[][] pts = new int[][] { new int[] { 2, 2 }, new int[] { 8, 8 }, new int[] { 26, 12 }, new int[] { 10, 19 } };
		int[] vs = new int[] { 1, 20, 60, 127 };
		int k = 0;
		foreach (int[] pt in pts)
			foreach (int v in vs)
			{
				using (Bitmap src = Load("blocks"))
				{
					Color c = (k % 2 == 0) ? Color.Red : Color.FromArgb(255, 10, 200, 30);
					using (Bitmap res = (Bitmap)fill.Invoke(null, new object[] { src, new Point(pt[0], pt[1]), c, v }))
					{
						string n = "fill_" + pt[0] + "_" + pt[1] + "_v" + v;
						Save(res, n);
						Rec(n, "fill", "blocks", pt[0].ToString(), pt[1].ToString(), c.ToArgb().ToString("X8"), v.ToString());
					}
				}
				k++;
			}
		foreach (string inp in new string[] { "blocks", "gray_ramp", "noise" })
			foreach (int[] pt in pts)
				using (Bitmap src = Load(inp))
				using (Bitmap res = (Bitmap)outl.Invoke(null, new object[] { src, new Point(pt[0], pt[1]) }))
				{
					string n = "outl_" + inp + "_" + pt[0] + "_" + pt[1];
					Save(res, n);
					Rec(n, "outline", inp, pt[0].ToString(), pt[1].ToString());
				}
	}

	static void GenRotFlip()
	{
		RotateFlipType[] ts = { RotateFlipType.Rotate90FlipNone, RotateFlipType.Rotate270FlipNone, RotateFlipType.RotateNoneFlipX, RotateFlipType.RotateNoneFlipY,
			RotateFlipType.Rotate180FlipNone, RotateFlipType.Rotate90FlipX };
		foreach (string inp in new string[] { "alpha_ramp", "grad_rgb" })
			foreach (RotateFlipType t in ts)
				using (Bitmap src = Load(inp))
				{
					src.RotateFlip(t);
					string n = "rf_" + inp + "_" + (int)t;
					Save(src, n);
					Rec(n, "rotflip", inp, ((int)t).ToString());
				}
	}

	static void GenResize()
	{
		InterpolationMode[] modes = { InterpolationMode.NearestNeighbor, InterpolationMode.Bilinear, InterpolationMode.Bicubic,
			InterpolationMode.HighQualityBilinear, InterpolationMode.HighQualityBicubic, InterpolationMode.Default, InterpolationMode.Low, InterpolationMode.High };
		int[][] sizes = { new int[] { 40, 30 }, new int[] { 100, 70 }, new int[] { 21, 17 } };
		foreach (InterpolationMode m in modes)
			foreach (int[] s in sizes)
				using (Bitmap src = Load("big_pattern"))
				using (Bitmap res = ImageTransform.ResizeImage(src, new Size(s[0], s[1]), false, m))
				{
					string n = "rs_" + m + "_" + s[0] + "x" + s[1];
					Save(res, n);
					Rec(n, "resize", "big_pattern", m.ToString(), s[0].ToString(), s[1].ToString());
				}
	}

	static int Main(string[] args)
	{
		root = args[0];
		inDir = Path.Combine(root, "input");
		outDir = Path.Combine(root, "output");
		Directory.CreateDirectory(inDir);
		Directory.CreateDirectory(outDir);
		MakeInputs();
		GenGray();
		GenWhitenize();
		GenThreshold();
		GenInvert();
		GenDither();
		GenFill();
		GenRotFlip();
		GenResize();
		File.WriteAllText(Path.Combine(root, "manifest.txt"), manifest.ToString());
		Console.WriteLine("ok");
		return 0;
	}
}

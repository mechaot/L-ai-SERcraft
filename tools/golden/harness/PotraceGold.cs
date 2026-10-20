// potrace, potrace_curves, potrace_rasterfill oracle.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Text;
using CsPotrace;
using LaserGRBL;
using LaserGRBL.RasterConverter;

namespace Golden
{
	public static class PotraceGold
	{
		class P
		{
			public string name; public bool useSpot; public int spot; public bool useSmooth; public decimal smooth; public bool useOpt; public decimal opt; public bool fast;
			public P(string n, bool us, int s, bool usm, decimal sm, bool uo, decimal o, bool f) { name = n; useSpot = us; spot = s; useSmooth = usm; smooth = sm; useOpt = uo; opt = o; fast = f; }
		}

		static P[] Params = new P[] {
			new P("P0", true, 2,  true,  1.0m,    true,  0.2m, false),
			new P("P1", false, 2, false, 1.0m,    false, 0.2m, false),
			new P("P2", true, 10, true,  1.3333m, true,  1.0m, true),
			new P("P3", true, 0,  true,  0.5m,    false, 0.2m, true),
			new P("P4", true, 5,  false, 1.0m,    true,  0.5m, false),
		};

		static ImageProcessor.Direction[] NewDirs = new ImageProcessor.Direction[] {
			ImageProcessor.Direction.None,
			ImageProcessor.Direction.NewHorizontal, ImageProcessor.Direction.NewVertical, ImageProcessor.Direction.NewDiagonal,
			ImageProcessor.Direction.NewReverseDiagonal, ImageProcessor.Direction.NewGrid, ImageProcessor.Direction.NewDiagonalGrid,
			ImageProcessor.Direction.NewCross, ImageProcessor.Direction.NewDiagonalCross, ImageProcessor.Direction.NewSquares,
			ImageProcessor.Direction.NewZigZag, ImageProcessor.Direction.NewHilbert, ImageProcessor.Direction.NewInsetFilling,
		};

		class Scn
		{
			public string id; public string image; public ImageProcessor.Direction dir; public P p; public bool pwm, noskip;
			public double res, fres; public float ox, oy; public int maxPower; public string lOn; public string conf;
		}

		public static void Run()
		{
			string[] areas = { "potrace", "potrace_curves", "potrace_rasterfill" };
			foreach (string a in areas) { string d = Path.Combine(H.GoldenRoot, a); if (Directory.Exists(d)) Directory.Delete(d, true); }

			List<string> bw = Images.BwNames();
			List<Scn> main = new List<Scn>();
			// A: every image x every vector fill dir, params P0, defaults
			foreach (string im in bw)
				foreach (ImageProcessor.Direction d in NewDirs)
					main.Add(Mk(im, d, Params[0], true, false, 10, 3, 0, 0, 1000, "M3", "default"));
			// B: no fill, other param sets
			for (int i = 1; i < Params.Length; i++)
				foreach (string im in bw)
					main.Add(Mk(im, ImageProcessor.Direction.None, Params[i], true, false, 10, 3, 0, 0, 1000, "M3", "default"));
			// C: horizontal + inset fill, P2, pwm off, offsets, other res
			foreach (string im in bw)
			{
				main.Add(Mk(im, ImageProcessor.Direction.NewHorizontal, Params[2], false, false, 7.3, 2, 12.5f, -3.25f, 255, "M4", "v11a"));
				main.Add(Mk(im, ImageProcessor.Direction.NewInsetFilling, Params[2], true, true, 7.3, 2, 12.5f, -3.25f, 255, "M3", "default"));
			}
			// D: pwm off / G0 skip off on a few images, several dirs
			string[] few = { "square_16x16", "ring_24x24", "two_blobs_28x14", "letter_O_hole_20x20", "triangle_24x20", "cross_21x21" };
			ImageProcessor.Direction[] dd = { ImageProcessor.Direction.None, ImageProcessor.Direction.NewGrid, ImageProcessor.Direction.NewDiagonal };
			foreach (string im in few)
				foreach (ImageProcessor.Direction d in dd)
				{
					main.Add(Mk(im, d, Params[0], false, false, 10, 3, 0, 0, 1000, "M3", "default"));
					main.Add(Mk(im, d, Params[3], true, true, 10, 3, 5, 5, 1000, "M4", "v11b"));
				}
			int n = 0;
			List<string> index = new List<string>();
			foreach (Scn s in main) { Potrace1("potrace", s, index); n++; }
			H.WriteLines("potrace", "index.txt", index);
			H.WriteConfs("potrace");
			Console.WriteLine("potrace cases: " + n);

			// raster fill (GDI+ resampling => non exact)
			List<Scn> rf = new List<Scn>();
			string[] rfi = { "square_16x16", "ring_24x24", "disc_20x20", "triangle_24x20" };
			ImageProcessor.Direction[] rfd = { ImageProcessor.Direction.Horizontal, ImageProcessor.Direction.Vertical, ImageProcessor.Direction.Diagonal };
			foreach (string im in rfi)
				foreach (ImageProcessor.Direction d in rfd)
				{
					rf.Add(Mk(im, d, Params[0], true, false, 10, 3, 0, 0, 1000, "M3", "default"));
					rf.Add(Mk(im, d, Params[2], false, false, 10, 3, 2, 1, 255, "M3", "v11a"));
				}
			List<string> idx2 = new List<string>();
			foreach (Scn s in rf) { Potrace1("potrace_rasterfill", s, idx2); }
			H.WriteLines("potrace_rasterfill", "index.txt", idx2);
			H.WriteConfs("potrace_rasterfill");
			Console.WriteLine("potrace_rasterfill cases: " + rf.Count);

			// curves
			List<string> idx3 = new List<string>();
			int nc = 0;
			foreach (string im in bw)
				foreach (P p in Params) { Curves(im, p, idx3); nc++; }
			string[] gr = { "circle_24x24", "noise_16x16", "gray_ramp_16x4", "alpha_ramp_12x3" };
			foreach (string im in gr)
				foreach (int pi in new int[] { 0, 2 }) { Curves(im, Params[pi], idx3); nc++; }
			H.WriteLines("potrace_curves", "index.txt", idx3);
			Console.WriteLine("potrace_curves cases: " + nc);
		}

		static Scn Mk(string im, ImageProcessor.Direction d, P p, bool pwm, bool noskip, double res, double fres, float ox, float oy, int maxP, string lOn, string conf)
		{
			Scn s = new Scn();
			s.image = im; s.dir = d; s.p = p; s.pwm = pwm; s.noskip = noskip; s.res = res; s.fres = fres; s.ox = ox; s.oy = oy; s.maxPower = maxP; s.lOn = lOn; s.conf = conf;
			s.id = im + "__" + d + "__" + p.name + (pwm ? "" : "__nopwm") + (noskip ? "__noskip" : "") + (res != 10 ? "__r73" : "") + (ox != 0 || oy != 0 ? "__off" : "") + (conf != "default" ? "__" + conf : "");
			return s;
		}

		static void Potrace1(string area, Scn s, List<string> index)
		{
			string inDir = Path.Combine(H.GoldenRoot, area, "inputs");
			H.ResetSettings();
			H.ApplyConf(s.conf);
			H.Set("Support Hardware PWM", s.pwm);
			H.Set("Disable G0 fast skip", s.noskip);
			index.Add(s.id);
			using (Bitmap bmp = Images.Reload(inDir, s.image))
			{
				GrblFile.L2LConf c = new GrblFile.L2LConf();
				c.res = s.res; c.fres = s.fres; c.oX = s.ox; c.oY = s.oy;
				c.markSpeed = 1000; c.borderSpeed = 800;
				c.minPower = 0; c.maxPower = s.maxPower;
				c.lOn = s.lOn; c.lOff = "M5";
				c.dir = s.dir; c.pwm = s.pwm; c.firmwareType = Firmware.Grbl;

				H.KV kv = new H.KV();
				kv.Add("image", s.image + ".png");
				kv.Add("width", bmp.Width); kv.Add("height", bmp.Height);
				kv.Add("dir", s.dir.ToString());
				kv.Add("UseSpotRemoval", H.B(s.p.useSpot)); kv.Add("SpotRemoval", s.p.spot);
				kv.Add("UseSmoothing", H.B(s.p.useSmooth)); kv.Add("Smoothing", H.D(s.p.smooth));
				kv.Add("UseOptimize", H.B(s.p.useOpt)); kv.Add("Optimize", H.D(s.p.opt));
				kv.Add("useOptimizeFast", H.B(s.p.fast));
				kv.Add("res", H.R(s.res)); kv.Add("fres", H.R(s.fres));
				kv.Add("oX", H.RF(s.ox)); kv.Add("oY", H.RF(s.oy));
				kv.Add("markSpeed", c.markSpeed); kv.Add("borderSpeed", c.borderSpeed);
				kv.Add("minPower", c.minPower); kv.Add("maxPower", c.maxPower);
				kv.Add("lOn", c.lOn); kv.Add("lOff", c.lOff);
				kv.Add("c.pwm", H.B(c.pwm));
				kv.Add("setting.Support Hardware PWM", H.B(s.pwm));
				kv.Add("setting.Disable G0 fast skip", H.B(s.noskip));
				kv.Add("grbl_conf", s.conf);
				kv.Add("grbl_conf.MaxPWM", H.D(GrblCore.Configuration.MaxPWM));

				GrblFile f = new GrblFile();
				string ex = "none";
				bool replica = GrblFile.RasterFilling(s.dir);
				kv.Add("harness_replica", H.B(replica));
				try { if (replica) RasterReplica(f, bmp, s, c); else f.LoadImagePotrace(bmp, "test", s.p.useSpot, s.p.spot, s.p.useSmooth, s.p.smooth, s.p.useOpt, s.p.opt, s.p.fast, c, false, null); }
				catch (Exception e) { ex = "EXCEPTION: " + H.ExName(e); if (Environment.GetEnvironmentVariable("GOLDEN_VERBOSE") != null) Console.Error.WriteLine(s.id + ": " + e); }
				kv.Add("exception", ex);
				kv.Add("count", f.Count);
				kv.Add("estimated_ticks", f.EstimatedTime.Ticks);
				kv.Add("drawing_range", H.RangeStr(f.Range.DrawingRange));
				kv.Add("moving_range", H.RangeStr(f.Range.MovingRange));
				kv.Add("quadrant", f.Quadrant.ToString());
				StringBuilder sb = new StringBuilder(kv.ToString());
				sb.Append("---\n");
				for (int i = 0; i < f.Count; i++) sb.Append(f[i].Command + "\n");
				H.Write(area, "cases/" + s.id + ".txt", sb.ToString());
			}
		}

		// libgdiplus: new Bitmap(w,h) has 0 dpi, so the original ImageTransform.ResizeImage always throws
		// (SetResolution(0,0)) on Mono. Same code with dpi forced to 96 (Windows default).
		static Bitmap ResizeFixed(Image image, Size size, bool killalfa, System.Drawing.Drawing2D.InterpolationMode interpolation)
		{
			Rectangle destRect = new Rectangle(0, 0, size.Width, size.Height);
			Bitmap destImage = new Bitmap(size.Width, size.Height);
			destImage.SetResolution(96, 96);
			using (Graphics g = Graphics.FromImage(destImage))
			{
				if (killalfa) g.Clear(Color.White);
				g.CompositingMode = killalfa ? System.Drawing.Drawing2D.CompositingMode.SourceOver : System.Drawing.Drawing2D.CompositingMode.SourceCopy;
				g.CompositingQuality = System.Drawing.Drawing2D.CompositingQuality.HighQuality;
				g.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.HighQuality;
				g.InterpolationMode = interpolation;
				g.PixelOffsetMode = System.Drawing.Drawing2D.PixelOffsetMode.HighQuality;
				using (System.Drawing.Imaging.ImageAttributes wrapMode = new System.Drawing.Imaging.ImageAttributes())
				{
					wrapMode.SetWrapMode(System.Drawing.Drawing2D.WrapMode.TileFlipXY);
					g.DrawImage(image, destRect, 0, 0, image.Width, image.Height, GraphicsUnit.Pixel, wrapMode);
				}
			}
			return destImage;
		}

		// Replica of GrblFile.LoadImagePotrace for Horizontal/Vertical/Diagonal (RasterFilling) dirs, using the original
		// private pieces by reflection (ImageLine2Line, OptimizePaths, Analyze), only ResizeImage swapped for ResizeFixed.
		static void RasterReplica(GrblFile f, Bitmap bmp, Scn s, GrblFile.L2LConf c)
		{
			typeof(GrblFile).GetField("skipcmd", H.ALL).SetValue(f, Settings.GetObject("Disable G0 fast skip", false) ? "G1" : "G0");
			string skipcmd = (string)typeof(GrblFile).GetField("skipcmd", H.ALL).GetValue(f);
			bmp.RotateFlip(RotateFlipType.RotateNoneFlipY);
			List<GrblCommand> list = f.Commands;
			Potrace.turdsize = (int)(s.p.useSpot ? s.p.spot : 2);
			Potrace.alphamax = s.p.useSmooth ? (double)s.p.smooth : 0.0;
			Potrace.opttolerance = s.p.useOpt ? (double)s.p.opt : 0.2;
			Potrace.curveoptimizing = s.p.useOpt;
			List<List<Curve>> plist = Potrace.PotraceTrace(bmp);

			using (Bitmap ptb = new Bitmap(bmp.Width, bmp.Height))
			{
				using (Graphics g = Graphics.FromImage(ptb))
				{
					double inset = Math.Max(1, c.res / c.fres);
					Potrace.Export2GDIPlus(plist, g, Brushes.Black, null, inset);
					using (Bitmap resampled = ResizeFixed(ptb, new Size((int)(bmp.Width * c.fres / c.res) + 1, (int)(bmp.Height * c.fres / c.res) + 1), true, System.Drawing.Drawing2D.InterpolationMode.HighQualityBicubic))
					{
						if (c.pwm) list.Add(new GrblCommand(String.Format("{0} S0", c.lOn)));
						else list.Add(new GrblCommand(String.Format("{0} S{1}", c.lOff, GrblCore.Configuration.MaxPWM)));
						list.Add(new GrblCommand(String.Format("F{0}", c.markSpeed)));
						c.vectorfilling = true;
						H.Invoke(f, "ImageLine2Line", resampled, c);
						list.Add(new GrblCommand(c.lOff));
					}
				}
			}

			bool supportPWM = Settings.GetObject("Support Hardware PWM", true);
			if (supportPWM) list.Add(new GrblCommand(c.lOn + " S0"));
			else list.Add(new GrblCommand(c.lOff + " S" + GrblCore.Configuration.MaxPWM));

			if (s.p.fast) plist = (List<List<Curve>>)H.Invoke(f, "OptimizePaths", plist, 0.0);
			else plist.Reverse();

			List<string> gc = new List<string>();
			if (supportPWM) gc.AddRange(Potrace.Export2GCode(plist, c.oX, c.oY, c.res, "S" + c.maxPower, "S0", bmp.Size, skipcmd));
			else gc.AddRange(Potrace.Export2GCode(plist, c.oX, c.oY, c.res, c.lOn, c.lOff, bmp.Size, skipcmd));
			list.Add(new GrblCommand(String.Format("F{0}", c.borderSpeed)));
			foreach (string code in gc) list.Add(new GrblCommand(code));
			if (supportPWM) list.Add(new GrblCommand(c.lOff));
			H.Invoke(f, "Analyze");
		}

		static string Pt(dPoint p) { return p == null ? "null,null" : H.R(p.X) + "," + H.R(p.Y); }

		static void Curves(string image, P p, List<string> index)
		{
			string id = image + "__" + p.name;
			index.Add(id);
			string inDir = Path.Combine(H.GoldenRoot, "potrace_curves", "inputs");
			using (Bitmap bmp = Images.Reload(inDir, image))
			{
				// same static setup as GrblFile.LoadImagePotrace
				Potrace.turdsize = (int)(p.useSpot ? p.spot : 2);
				Potrace.alphamax = p.useSmooth ? (double)p.smooth : 0.0;
				Potrace.opttolerance = p.useOpt ? (double)p.opt : 0.2;
				Potrace.curveoptimizing = p.useOpt;

				H.KV kv = new H.KV();
				kv.Add("image", image + ".png");
				kv.Add("width", bmp.Width); kv.Add("height", bmp.Height);
				kv.Add("turdsize", Potrace.turdsize);
				kv.Add("alphamax", H.R(Potrace.alphamax));
				kv.Add("opttolerance", H.R(Potrace.opttolerance));
				kv.Add("curveoptimizing", H.B(Potrace.curveoptimizing));
				kv.Add("turnpolicy", Potrace.turnpolicy.ToString());
				kv.Add("Treshold", H.R(Potrace.Treshold));
				List<List<Curve>> pl = null;
				string ex = "none";
				try { pl = Potrace.PotraceTrace(bmp); }
				catch (Exception e) { ex = "EXCEPTION: " + H.ExName(e); }
				kv.Add("exception", ex);
				kv.Add("paths", pl == null ? 0 : pl.Count);
				StringBuilder sb = new StringBuilder(kv.ToString());
				sb.Append("---\n");
				if (pl != null)
				{
					for (int i = 0; i < pl.Count; i++)
					{
						sb.Append("path " + i + " " + pl[i].Count + "\n");
						foreach (Curve c in pl[i])
							sb.Append(c.Kind + " " + Pt(c.A) + " " + Pt(c.ControlPointA) + " " + Pt(c.ControlPointB) + " " + Pt(c.B) + "\n");
					}
				}
				H.Write("potrace_curves", "cases/" + id + ".txt", sb.ToString());
			}
		}
	}
}

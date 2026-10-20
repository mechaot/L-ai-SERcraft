// l2l: GrblFile.LoadImageL2L oracle.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Text;
using LaserGRBL;
using LaserGRBL.RasterConverter;

namespace Golden
{
	public static class L2L
	{
		class T
		{
			public ImageProcessor.Direction dir; public bool pwm, uni, noskip; public Firmware fw;
			public double res; public float ox, oy; public int minP, maxP; public string lOn; public string conf;
			public T(ImageProcessor.Direction d, bool pwm, bool uni, bool noskip, Firmware fw, double res, float ox, float oy, int minP, int maxP, string lOn, string conf)
			{ dir = d; this.pwm = pwm; this.uni = uni; this.noskip = noskip; this.fw = fw; this.res = res; this.ox = ox; this.oy = oy; this.minP = minP; this.maxP = maxP; this.lOn = lOn; this.conf = conf; }
		}

		static T[] Templates()
		{
			var H_ = ImageProcessor.Direction.Horizontal; var V_ = ImageProcessor.Direction.Vertical; var D_ = ImageProcessor.Direction.Diagonal;
			return new T[] {
				new T(H_, true,  false, false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(V_, true,  false, false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(D_, true,  false, false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(H_, false, false, false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(V_, false, false, false, Firmware.Grbl,     10,  0,     0,     0,  255, "M4", "default"),
				new T(D_, false, false, false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(H_, true,  true,  false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(V_, true,  true,  true,  Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(D_, false, true,  false, Firmware.Grbl,     10,  0,     0,     0,  255, "M3", "default"),
				new T(H_, true,  false, false, Firmware.Smoothie, 7.3, 12.5f, -3.25f, 10, 200, "M3", "default"),
				new T(V_, true,  false, true,  Firmware.Grbl,     7.3, 12.5f, -3.25f, 20, 255, "M3", "v11a"),
				new T(D_, true,  false, false, Firmware.Smoothie, 7.3, 12.5f, -3.25f, 0,  100, "M4", "default"),
				new T(H_, false, false, true,  Firmware.Grbl,     10,  12.5f, -3.25f, 0,  255, "M4", "v11a"),
				new T(D_, true,  true,  true,  Firmware.Grbl,     7.3, 12.5f, -3.25f, 5,  250, "M3", "v11b"),
			};
		}

		public static void Run()
		{
			string area = "l2l";
			string inDir = Path.Combine(H.GoldenRoot, area, "inputs");
			string caseDir = Path.Combine(H.GoldenRoot, area, "cases");
			if (Directory.Exists(inDir)) Directory.Delete(inDir, true);

			T[] ts = Templates();
			List<string> index = new List<string>();
			List<string> names = Images.GrayNames();
			int n = 0;
			for (int im = 0; im < names.Count; im++)
			{
				for (int k = 0; k < 10; k++)
				{
					int ti = (im * 3 + k) % ts.Length;
					T t = ts[ti];
					string id = names[im] + "__t" + ti.ToString("00");
					index.Add(id);
					Case(area, inDir, names[im], id, t);
					n++;
				}
			}
			H.WriteLines(area, "index.txt", index);
			H.WriteConfs(area);
			Console.WriteLine("l2l cases: " + n);
		}

		static void Case(string area, string inDir, string image, string id, T t)
		{
			H.ResetSettings();
			H.ApplyConf(t.conf);
			H.Set("Unidirectional Engraving", t.uni);
			H.Set("Disable G0 fast skip", t.noskip);

			// fresh bitmap per case; original mutates it (RotateFlip) -> PNG on disk is the pre-call input
			using (Bitmap bmp = Images.Reload(inDir, image))
			{
				GrblFile.L2LConf c = new GrblFile.L2LConf();
				c.res = t.res; c.fres = t.res; c.oX = t.ox; c.oY = t.oy;
				c.markSpeed = 1000; c.borderSpeed = 1000;
				c.minPower = t.minP; c.maxPower = t.maxP;
				c.lOn = t.lOn; c.lOff = "M5";
				c.dir = t.dir; c.pwm = t.pwm; c.firmwareType = t.fw;

				H.KV kv = new H.KV();
				kv.Add("image", image + ".png");
				kv.Add("width", bmp.Width); kv.Add("height", bmp.Height);
				kv.Add("dir", t.dir.ToString());
				kv.Add("pwm", H.B(t.pwm));
				kv.Add("firmwareType", t.fw.ToString());
				kv.Add("res", H.R(t.res));
				kv.Add("fres", H.R(c.fres));
				kv.Add("oX", H.RF(t.ox));
				kv.Add("oY", H.RF(t.oy));
				kv.Add("minPower", t.minP); kv.Add("maxPower", t.maxP);
				kv.Add("markSpeed", c.markSpeed); kv.Add("borderSpeed", c.borderSpeed);
				kv.Add("lOn", t.lOn); kv.Add("lOff", "M5");
				kv.Add("setting.Unidirectional Engraving", H.B(t.uni));
				kv.Add("setting.Disable G0 fast skip", H.B(t.noskip));
				kv.Add("grbl_conf", t.conf);
				kv.Add("grbl_conf.MaxPWM", H.D(GrblCore.Configuration.MaxPWM));
				kv.Add("grbl_conf.MaxRateX", H.D(GrblCore.Configuration.MaxRateX));

				GrblFile f = new GrblFile();
				string ex = "none";
				try { f.LoadImageL2L(bmp, "test", c, false, null); }
				catch (Exception e) { ex = "EXCEPTION: " + H.ExName(e); }
				kv.Add("exception", ex);
				kv.Add("count", f.Count);
				kv.Add("estimated_ticks", f.EstimatedTime.Ticks);
				kv.Add("drawing_range", H.RangeStr(f.Range.DrawingRange));
				kv.Add("moving_range", H.RangeStr(f.Range.MovingRange));
				kv.Add("quadrant", f.Quadrant.ToString());
				StringBuilder sb = new StringBuilder(kv.ToString());
				sb.Append("---\n");
				for (int i = 0; i < f.Count; i++) sb.Append(f[i].Command + "\n");
				H.Write(area, "cases/" + id + ".txt", sb.ToString());
			}
		}
	}
}

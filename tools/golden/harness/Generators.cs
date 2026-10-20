// generators: GenerateCuttingTest / GenerateGreyscaleTest / GenerateShakeTest oracle.
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using LaserGRBL;

namespace Golden
{
	public static class Generators
	{
		public static void Run()
		{
			string area = "generators";
			string root = Path.Combine(H.GoldenRoot, area);
			if (Directory.Exists(root)) Directory.Delete(root, true);
			List<string> idx = new List<string>();

			// CuttingTest(f_col, f_start, f_end, p_start, p_end, s_fixed, f_text, s_text, title, ton)
			object[][] cut = {
				new object[] { 3, 1000, 3000, 1, 3, 255, 1000, 100, "", "M3", "default" },
				new object[] { 1, 500, 500, 1, 1, 1000, 800, 500, "wood", "M4", "default" },
				new object[] { 5, 300, 1500, 2, 4, 700, 1500, 200, "My Title", "M3", "v11a" },
				new object[] { 2, 100, 100, 3, 3, 255, 1000, 255, "", "M4", "default" },
				new object[] { 4, 2000, 500, 1, 2, 500, 2000, 100, "reverse feed", "M3", "default" },
				new object[] { 6, 600, 3100, 1, 5, 1000, 1200, 800, "big [x] test", "M3", "v11b" },
				new object[] { 3, 1000, 2000, 5, 5, 255, 1000, 100, "", "M3", "default" },
			};
			int n = 0;
			foreach (object[] a in cut)
			{
				string id = "cutting_" + n.ToString("00");
				Gen(area, id, a[10].ToString(), "GenerateCuttingTest(f_col=" + a[0] + ", f_start=" + a[1] + ", f_end=" + a[2] + ", p_start=" + a[3] + ", p_end=" + a[4] + ", s_fixed=" + a[5] + ", f_text=" + a[6] + ", s_text=" + a[7] + ", title=\"" + a[8] + "\", ton=\"" + a[9] + "\")",
					delegate (GrblFile f) { H.Invoke(f, "GenerateCuttingTest", a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9]); });
				idx.Add(id); n++;
			}

			// GreyscaleTest(f_row, s_col, f_start, f_end, s_start, s_end, x_size, y_size, resolution, f_grid, s_grid, title, f_text, s_text, ton)
			object[][] grey = {
				new object[] { 3, 4, 1000, 3000, 100, 1000, 40, 30, 2.0, 1000, 500, "", 1000, 300, "M3", "default" },
				new object[] { 1, 1, 1000, 1000, 255, 255, 20, 10, 1.0, 500, 100, "single", 800, 255, "M4", "default" },
				new object[] { 5, 5, 500, 2000, 0, 255, 50, 50, 1.5, 1000, 255, "mid", 1500, 200, "M3", "v11a" },
				new object[] { 2, 3, 3000, 1000, 1000, 100, 30, 20, 3.3, 1000, 1000, "res3.3", 1000, 100, "M3", "default" },
				new object[] { 4, 2, 800, 800, 10, 500, 25, 40, 4.0, 2000, 50, "", 600, 150, "M4", "v11b" },
				new object[] { 3, 3, 100, 900, 100, 900, 45, 35, 0.7, 1000, 300, "slow", 1000, 300, "M3", "default" },
			};
			n = 0;
			foreach (object[] a in grey)
			{
				string id = "greyscale_" + n.ToString("00");
				Gen(area, id, a[15].ToString(), "GenerateGreyscaleTest(f_row=" + a[0] + ", s_col=" + a[1] + ", f_start=" + a[2] + ", f_end=" + a[3] + ", s_start=" + a[4] + ", s_end=" + a[5] + ", x_size=" + a[6] + ", y_size=" + a[7] + ", resolution=" + H.R((double)a[8]) + ", f_grid=" + a[9] + ", s_grid=" + a[10] + ", title=\"" + a[11] + "\", f_text=" + a[12] + ", s_text=" + a[13] + ", ton=\"" + a[14] + "\")",
					delegate (GrblFile f) { H.Invoke(f, "GenerateGreyscaleTest", a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14]); });
				idx.Add(id); n++;
			}

			// ShakeTest(axis, flimit, axislen, cpower, cspeed)
			object[][] shake = {
				new object[] { "X", 1000, 100, 500, 1000, "default" },
				new object[] { "Y", 3000, 50, 255, 500, "default" },
				new object[] { "X", 5000, 200, 1000, 3000, "v11a" },
				new object[] { "Y", 2000, 30, 100, 2000, "default" },
				new object[] { "X", 8000, 410, 800, 1500, "default" },
			};
			n = 0;
			foreach (object[] a in shake)
			{
				string id = "shake_" + n.ToString("00");
				Gen(area, id, a[5].ToString(), "GenerateShakeTest(axis=\"" + a[0] + "\", flimit=" + a[1] + ", axislen=" + a[2] + ", cpower=" + a[3] + ", cspeed=" + a[4] + ")",
					delegate (GrblFile f) { H.Invoke(f, "GenerateShakeTest", a[0], a[1], a[2], a[3], a[4]); });
				idx.Add(id); n++;
			}
			H.WriteLines(area, "index.txt", idx);
			H.WriteConfs(area);
			Console.WriteLine("generators cases: " + idx.Count);
		}

		static void Gen(string area, string id, string conf, string call, Action<GrblFile> act)
		{
			H.ResetSettings();
			H.ApplyConf(conf);
			GrblFile f = new GrblFile();
			H.KV kv = new H.KV();
			kv.Add("call", call);
			kv.Add("grbl_conf", conf);
			kv.Add("grbl_conf.MaxRateX", H.D(GrblCore.Configuration.MaxRateX));
			string ex = "none";
			try { act(f); }
			catch (Exception e) { ex = "EXCEPTION: " + H.ExName(e); }
			kv.Add("exception", ex);
			kv.Add("count", f.Count);
			kv.Add("estimated_ticks", f.EstimatedTime.Ticks);
			kv.Add("drawing_range", H.RangeStr(f.Range.DrawingRange));
			kv.Add("moving_range", H.RangeStr(f.Range.MovingRange));
			kv.Add("quadrant", f.Quadrant.ToString());
			StringBuilder sb = new StringBuilder(kv.ToString());
			sb.Append("---\n");
			for (int i = 0; i < f.Count; i++) sb.Append(i + "\t" + f[i].TimeOffset.Ticks + "\t" + f[i].Command + "\n");
			H.Write(area, "cases/" + id + ".txt", sb.ToString());
		}
	}
}

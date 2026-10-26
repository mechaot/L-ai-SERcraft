// svg: SvgConverter.GCodeFromSVG.convertFromText oracle (class is internal -> reflection).
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Reflection;
using System.Text;
using LaserGRBL;
using LaserGRBL.SvgConverter;

namespace Golden
{
	public static class SvgGold
	{
		class V
		{
			public string name;
			public bool pwm = true; public bool noskip = false; public int borderSpeed = 1000; public int powerMax = 255;
			public string lOn = "M3"; public string lOff = "M5"; public Firmware fw = Firmware.Grbl; public string conf = "default";
			public bool scaleApply = false; public float maxSize = 100; public float offX = 0, offY = 0;
			public ColorFilter filter = ColorFilter.All; public bool importMM = true;
			public V(string n) { name = n; }
		}

		static List<V> Variants()
		{
			List<V> v = new List<V>();
			v.Add(new V("pwm0") { pwm = false });
			v.Add(new V("pwm0_v11a_m4") { pwm = false, conf = "v11a", lOn = "M4" });
			v.Add(new V("powermax1000") { powerMax = 1000 });
			v.Add(new V("smoothie") { fw = Firmware.Smoothie });
			v.Add(new V("smoothie_pm100") { fw = Firmware.Smoothie, powerMax = 100 });
			v.Add(new V("noskip") { noskip = true });
			v.Add(new V("speed3000") { borderSpeed = 3000 });
			v.Add(new V("scale50_off") { scaleApply = true, maxSize = 50, offX = 5, offY = -3 });
			v.Add(new V("offset_only") { offX = 12.5f, offY = -3.25f });
			return v;
		}

		public static void Run()
		{
			string area = "svg";
			string root = Path.Combine(H.GoldenRoot, area);
			if (Directory.Exists(root)) Directory.Delete(root, true);
			Directory.CreateDirectory(Path.Combine(root, "inputs"));

			// inputs
			List<string> svgs = new List<string>();
			string[] files = Directory.GetFiles(Path.Combine(H.InputsRoot, "svg"), "*.svg");
			Array.Sort(files, StringComparer.Ordinal);
			foreach (string f in files) { string n = Path.GetFileName(f); File.Copy(f, Path.Combine(root, "inputs", n), true); svgs.Add(n); }
			string acc = Path.Combine(H.RepoRoot, "resources", "generator", "LaserGRBL-accuracy-test-file.svg");
			File.Copy(acc, Path.Combine(root, "inputs", "LaserGRBL-accuracy-test-file.svg"), true);
			svgs.Add("LaserGRBL-accuracy-test-file.svg");

			StringBuilder idx = new StringBuilder();
			idx.Append("id\tsvg\tlegacyBezier\tvariant\tstatus\n");
			int n2 = 0;

			Func<string, V, bool, string> run = delegate (string svg, V v, bool legacy)
			{
				string id = svg.Replace(".svg", "") + "__" + v.name + "__bz" + (legacy ? "1" : "0");
				string status = RunOne(root, id, svg, v, legacy);
				idx.Append(id + "\t" + svg + "\t" + (legacy ? "true" : "false") + "\t" + v.name + "\t" + status + "\n");
				n2++;
				return id;
			};

			V def = new V("default");
			foreach (string svg in svgs)
				foreach (bool legacy in new bool[] { false, true })
					run(svg, def, legacy);

			string[] varSvgs = { "path_arc_abs.svg", "path_cubic_abs.svg", "shape_mixed.svg", "tf_nested_groups.svg", "shape_rect_rx.svg", "dim_mm_viewbox.svg", "shape_ellipse.svg", "path_quad_smooth_abs.svg" };
			foreach (V v in Variants())
				foreach (string svg in varSvgs)
					foreach (bool legacy in new bool[] { false, true })
						run(svg, v, legacy);

			ColorFilter[] filters = { ColorFilter.Red, ColorFilter.Green, ColorFilter.Blue, ColorFilter.Black };
			string[] colSvgs = { "color_mixed.svg", "color_style_fill_only.svg", "color_invalid_rgb_func.svg", "color_style_all_valid.svg", "color_style_invalid_name.svg", "color_style_dup_keys.svg", "color_style_attr_mix.svg" };
			foreach (ColorFilter cf in filters)
				foreach (string svg in colSvgs)
					run(svg, new V("filter" + cf) { filter = cf }, false);
			run("color_mixed.svg", new V("importMM_false") { importMM = false }, false);
			run("LaserGRBL-accuracy-test-file.svg", new V("pwm0_v11a") { pwm = false, conf = "v11a" }, false);
			run("LaserGRBL-accuracy-test-file.svg", new V("scale50_off") { scaleApply = true, maxSize = 50, offX = 5, offY = -3 }, false);

			H.Write(area, "cases/index.tsv", idx.ToString());
			H.WriteConfs(area);
			Console.WriteLine("svg cases: " + n2);
		}

		static string RunOne(string root, string id, string svg, V v, bool legacy)
		{
			H.ResetSettings();
			H.ApplyConf(v.conf);
			H.Set("Support Hardware PWM", v.pwm);
			H.Set("Disable G0 fast skip", v.noskip);
			H.Set("GrayScaleConversion.VectorizeOptions.BorderSpeed", v.borderSpeed);
			H.Set("GrayScaleConversion.Gcode.LaserOptions.PowerMax", v.powerMax);
			H.Set("GrayScaleConversion.Gcode.LaserOptions.LaserOn", v.lOn);
			H.Set("GrayScaleConversion.Gcode.LaserOptions.LaserOff", v.lOff);
			H.Set("Firmware Type", v.fw);
			H.Set("Vector.UseSmartBezier", !legacy);

			// gcode.firmwareType is a static initialiser, read once per process -> force it by reflection
			Type gcodeT = H.Asm.GetType("LaserGRBL.SvgConverter.gcode");
			gcodeT.GetField("firmwareType", H.ALL).SetValue(null, v.fw);

			Type t = H.Asm.GetType("LaserGRBL.SvgConverter.GCodeFromSVG");
			object conv = Activator.CreateInstance(t, true);
			// same property setup as GrblFile.LoadImportedSVG (+ centerline-style scale/offset for some variants)
			t.GetField("GCodeXYFeed", H.ALL).SetValue(conv, (float)Settings.GetObject("GrayScaleConversion.VectorizeOptions.BorderSpeed", 1000));
			t.GetProperty("UseLegacyBezier", H.ALL).SetValue(conv, legacy, null);
			if (v.scaleApply)
			{
				t.GetProperty("SvgScaleApply", H.ALL).SetValue(conv, true, null);
				t.GetProperty("SvgMaxSize", H.ALL).SetValue(conv, v.maxSize, null);
			}
			t.GetField("UserOffset", H.ALL).SetValue(conv, new PointF(v.offX, v.offY));

			string text = File.ReadAllText(Path.Combine(root, "inputs", svg), H.Utf8);
			string result; string status;
			try
			{
				result = (string)H.Invoke(conv, "convertFromText", text, null, v.importMM, v.filter);
				status = "ok";
			}
			catch (Exception e)
			{
				status = "EXCEPTION: " + H.ExName(e);
				result = status + "\n";
			}
			H.WriteRaw("svg", "cases/" + id + ".gcode", result);
			return status;
		}
	}
}

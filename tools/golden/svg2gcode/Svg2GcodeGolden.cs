// svg2gcode unit oracle: WPF Matrix ops, BezierTools, ConvertToPixel/ParseTransform/GetTextBetween,
// ColorTranslator.FromHtml, and extra full conversions (internal members via reflection).
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text;
using System.Windows;
using System.Windows.Media;
using LaserGRBL;
using LaserGRBL.SvgConverter;
using Point = System.Windows.Point;

static class G
{
	const BindingFlags ALL = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static;
	static CultureInfo Inv = CultureInfo.InvariantCulture;
	static Assembly Asm = typeof(GrblFile).Assembly;
	static string Out, InDir;

	static string D(double d)
	{
		if (double.IsNaN(d)) return "NaN";
		return BitConverter.DoubleToInt64Bits(d).ToString("x16");
	}
	static string F(float f)
	{
		if (float.IsNaN(f)) return "NaN";
		return BitConverter.ToInt32(BitConverter.GetBytes(f), 0).ToString("x8");
	}
	static double P(string s) { return double.Parse(s, Inv); }
	static string M(Matrix m) { return D(m.M11) + " " + D(m.M12) + " " + D(m.M21) + " " + D(m.M22) + " " + D(m.OffsetX) + " " + D(m.OffsetY) + " " + (m.IsIdentity ? "1" : "0"); }
	static string Ex(Exception e) { while (e is TargetInvocationException && e.InnerException != null) e = e.InnerException; return "EX:" + e.GetType().FullName; }

	static Matrix Mk(string[] t, int o) { return new Matrix(P(t[o]), P(t[o + 1]), P(t[o + 2]), P(t[o + 3]), P(t[o + 4]), P(t[o + 5])); }

	static void Matrices()
	{
		StringBuilder sb = new StringBuilder();
		int n = 0;
		foreach (string line in File.ReadAllLines(Path.Combine(Out, "matrix_ops.txt")))
		{
			if (line.Length == 0) continue;
			Matrix m = new Matrix(); m.SetIdentity();
			List<string> rec = new List<string>();
			foreach (string op in line.Split(';'))
			{
				string[] t = op.Split(' ');
				try
				{
					switch (t[0])
					{
						case "new": m = Mk(t, 1); break;
						case "ident": m.SetIdentity(); break;
						case "set":
							switch (t[1])
							{
								case "M11": m.M11 = P(t[2]); break;
								case "M12": m.M12 = P(t[2]); break;
								case "M21": m.M21 = P(t[2]); break;
								case "M22": m.M22 = P(t[2]); break;
								case "OffsetX": m.OffsetX = P(t[2]); break;
								case "OffsetY": m.OffsetY = P(t[2]); break;
							}
							break;
						case "scale": m.Scale(P(t[1]), P(t[2])); break;
						case "scaleat": m.ScaleAt(P(t[1]), P(t[2]), P(t[3]), P(t[4])); break;
						case "rotate": m.Rotate(P(t[1])); break;
						case "rotateat": m.RotateAt(P(t[1]), P(t[2]), P(t[3])); break;
						case "translate": m.Translate(P(t[1]), P(t[2])); break;
						case "skew": m.Skew(P(t[1]), P(t[2])); break;
						case "mul": m = Matrix.Multiply(m, Mk(t, 1)); break;
						case "mulr": m = Matrix.Multiply(Mk(t, 1), m); break;
						case "invert": m.Invert(); break;
						case "xf": { Point p = m.Transform(new Point(P(t[1]), P(t[2]))); rec.Add("P " + D(p.X) + " " + D(p.Y)); continue; }
						case "xv": { Vector v = m.Transform(new Vector(P(t[1]), P(t[2]))); rec.Add("V " + D(v.X) + " " + D(v.Y)); continue; }
					}
					rec.Add(M(m));
				}
				catch (Exception e) { rec.Add(Ex(e)); }
			}
			sb.Append(string.Join("|", rec.ToArray())).Append('\n');
			n++;
		}
		File.WriteAllText(Path.Combine(Out, "matrix.txt"), sb.ToString());
		Console.WriteLine("matrix lines: " + n);
	}

	static void Beziers()
	{
		Type bt = Asm.GetType("LaserGRBL.SvgConverter.BezierTools");
		Type gt = Asm.GetType("LaserGRBL.SvgConverter.GCodeFromSVG");
		object conv = Activator.CreateInstance(gt, true);
		StringBuilder sb = new StringBuilder();
		foreach (string line in File.ReadAllLines(Path.Combine(Out, "bezier_in.txt")))
		{
			if (line.Length == 0) continue;
			string[] f = line.Split('\t');
			string[] c = f[2].Split(' ');
			Point[] pts = new Point[c.Length / 2];
			for (int i = 0; i < pts.Length; i++) pts[i] = new Point(P(c[2 * i]), P(c[2 * i + 1]));
			List<string> rec = new List<string>();
			try
			{
				IEnumerable<Point> res;
				if (f[0] == "F")
					res = (IEnumerable<Point>)bt.GetMethod("FlattenTo", ALL).Invoke(null, new object[] { pts, P(f[3]), int.Parse(f[4]) });
				else
					res = (Point[])gt.GetMethod("GetBezierApproximationOld", ALL).Invoke(conv, new object[] { pts, int.Parse(f[4]) });
				foreach (Point p in res) rec.Add(D(p.X) + " " + D(p.Y));
			}
			catch (Exception e) { rec.Add(Ex(e)); }
			sb.Append(string.Join("|", rec.ToArray())).Append('\n');
		}
		File.WriteAllText(Path.Combine(Out, "bezier.txt"), sb.ToString());
	}

	static void Parsing()
	{
		Type gt = Asm.GetType("LaserGRBL.SvgConverter.GCodeFromSVG");
		StringBuilder sb = new StringBuilder();
		foreach (string line in File.ReadAllLines(Path.Combine(Out, "parse_in.txt")))
		{
			if (line.Length == 0) continue;
			string[] f = line.Split('\t');
			string res;
			try
			{
				switch (f[0])
				{
					case "px":
						{
							float ext = f.Length > 2 ? (float)P(f[2]) : 1f;
							res = F((float)gt.GetMethod("ConvertToPixel", ALL).Invoke(null, new object[] { f[1], ext }));
							break;
						}
					case "tf": res = M((Matrix)gt.GetMethod("ParseTransform", ALL, null, new Type[] { typeof(string) }, null).Invoke(null, new object[] { f[1] })); break;
					case "tb": res = "S:" + (string)gt.GetMethod("GetTextBetween", ALL).Invoke(null, new object[] { f[1], f[2] }); break;
					case "color":
						{
							Color c = ColorTranslator.FromHtml(f[1]);
							res = c.IsEmpty ? "EMPTY" : c.R + " " + c.G + " " + c.B;
							break;
						}
					default: throw new Exception("bad kind");
				}
			}
			catch (Exception e) { res = Ex(e); }
			sb.Append(res).Append('\n');
		}
		File.WriteAllText(Path.Combine(Out, "parse.txt"), sb.ToString());
	}

	class V
	{
		public string name; public bool pwm = true; public bool noskip; public int borderSpeed = 1000; public int powerMax = 255;
		public Firmware fw = Firmware.Grbl; public bool scale; public float maxSize = 100, offX, offY; public ColorFilter filter = ColorFilter.All;
		public bool toMM = true;
		public V(string n) { name = n; }
	}

	static void Conversions()
	{
		string root = Path.Combine(Out, "convert");
		if (Directory.Exists(root)) Directory.Delete(root, true);
		Directory.CreateDirectory(root);
		string inputs = Path.Combine(Out, "inputs");
		if (Directory.Exists(inputs)) Directory.Delete(inputs, true);
		Directory.CreateDirectory(inputs);
		string[] files = Directory.GetFiles(InDir, "*.svg");
		Array.Sort(files, StringComparer.Ordinal);
		StringBuilder idx = new StringBuilder("id\tsvg\tlegacyBezier\tvariant\tstatus\n");
		int count = 0;
		Func<string, V, bool, bool> run = delegate (string svg, V v, bool legacy)
		{
			string id = svg.Replace(".svg", "") + "__" + v.name + "__bz" + (legacy ? "1" : "0");
			string status = RunOne(root, inputs, id, svg, v, legacy);
			idx.Append(id + "\t" + svg + "\t" + (legacy ? "true" : "false") + "\t" + v.name + "\t" + status + "\n");
			count++;
			return true;
		};
		foreach (string f in files)
		{
			string svg = Path.GetFileName(f);
			File.Copy(f, Path.Combine(inputs, svg), true);
			run(svg, new V("default"), false);
			run(svg, new V("default"), true);
		}
		foreach (string svg in new[] { "tf_all.svg", "units_misc.svg", "svg_no_size.svg", "rect_variants.svg", "arcs_misc.svg" })
			run(svg, new V("mm_false") { toMM = false }, false);
		foreach (string svg in new[] { "tf_all.svg", "inkscape_attrs.svg", "arcs_misc.svg", "group_order.svg", "curves_misc.svg", "svg_no_size.svg", "viewbox_zero.svg" })
		{
			run(svg, new V("scale50_off") { scale = true, maxSize = 50, offX = 5, offY = -3 }, false);
			run(svg, new V("scale_mm_false") { scale = true, maxSize = 75, offX = -2.5f, offY = 1.25f, toMM = false }, true);
		}
		foreach (string svg in new[] { "style_colors.svg", "style_attr_colors.svg", "style_trailing_semicolon.svg", "group_order.svg", "inkscape_attrs.svg", "tf_all.svg" })
			foreach (ColorFilter cf in new[] { ColorFilter.Red, ColorFilter.Green, ColorFilter.Blue, ColorFilter.Black })
				run(svg, new V("filter" + cf) { filter = cf }, false);
		foreach (string svg in new[] { "curves_misc.svg", "arcs_misc.svg", "rect_variants.svg", "tf_all.svg" })
		{
			run(svg, new V("pwm0") { pwm = false }, false);
			run(svg, new V("smoothie") { fw = Firmware.Smoothie }, false);
			run(svg, new V("noskip_speed") { noskip = true, borderSpeed = 2500, powerMax = 640 }, true);
		}
		File.WriteAllText(Path.Combine(root, "index.tsv"), idx.ToString());
		Console.WriteLine("convert cases: " + count);
	}

	static string RunOne(string root, string inputs, string id, string svg, V v, bool legacy)
	{
		System.Collections.IDictionary dic = (System.Collections.IDictionary)typeof(Settings).GetField("dic", ALL).GetValue(null);
		dic.Clear();
		Settings.SetObject("Support Hardware PWM", v.pwm);
		Settings.SetObject("Disable G0 fast skip", v.noskip);
		Settings.SetObject("GrayScaleConversion.VectorizeOptions.BorderSpeed", v.borderSpeed);
		Settings.SetObject("GrayScaleConversion.Gcode.LaserOptions.PowerMax", v.powerMax);
		Settings.SetObject("GrayScaleConversion.Gcode.LaserOptions.LaserOn", "M3");
		Settings.SetObject("GrayScaleConversion.Gcode.LaserOptions.LaserOff", "M5");
		Settings.SetObject("Firmware Type", v.fw);
		Settings.SetObject("Vector.UseSmartBezier", !legacy);
		Type gcodeT = Asm.GetType("LaserGRBL.SvgConverter.gcode");
		gcodeT.GetField("firmwareType", ALL).SetValue(null, v.fw);
		Type t = Asm.GetType("LaserGRBL.SvgConverter.GCodeFromSVG");
		object conv = Activator.CreateInstance(t, true);
		t.GetProperty("UseLegacyBezier", ALL).SetValue(conv, legacy, null);
		t.GetField("svgConvertToMM", ALL).SetValue(conv, v.toMM);
		if (v.scale)
		{
			t.GetProperty("SvgScaleApply", ALL).SetValue(conv, true, null);
			t.GetProperty("SvgMaxSize", ALL).SetValue(conv, v.maxSize, null);
		}
		t.GetField("UserOffset", ALL).SetValue(conv, new PointF(v.offX, v.offY));
		string text = File.ReadAllText(Path.Combine(inputs, svg), new UTF8Encoding(false));
		string result, status;
		try
		{
			result = (string)t.GetMethod("convertFromText", ALL).Invoke(conv, new object[] { text, null, true, v.filter });
			status = "ok";
		}
		catch (Exception e)
		{
			while (e is TargetInvocationException && e.InnerException != null) e = e.InnerException;
			status = "EXCEPTION: " + e.GetType().FullName;
			result = status + "\n";
		}
		File.WriteAllText(Path.Combine(root, id + ".gcode"), result, new UTF8Encoding(false));
		return status;
	}

	static void Main(string[] a)
	{
		Out = a[0]; InDir = a[1];
		Matrices(); Beziers(); Parsing(); Conversions();
	}
}

// custom_code + custom_buttons: decode StandardButtons.zbn and evaluate custom-button expressions with the original GrblCore.EvaluateExpression.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Reflection;
using System.Runtime.Serialization;
using System.Text;
using LaserGRBL;

namespace Golden
{
	public static class CustomCode
	{
		static string J(string s)
		{
			if (s == null) return "null";
			StringBuilder sb = new StringBuilder("\"");
			foreach (char c in s)
			{
				if (c == '"') sb.Append("\\\"");
				else if (c == '\\') sb.Append("\\\\");
				else if (c == '\n') sb.Append("\\n");
				else if (c == '\r') sb.Append("\\r");
				else if (c == '\t') sb.Append("\\t");
				else if (c < 0x20) sb.Append("\\u" + ((int)c).ToString("x4"));
				else sb.Append(c);
			}
			return sb.Append('"').ToString();
		}

		static List<CustomButton> Buttons;

		public static void Run()
		{
			string src = Path.Combine(H.RepoRoot, "resources", "misc", "StandardButtons.zbn");
			// ObjFromFile REWRITES the file it reads (re-serialises) -> work on a scratch copy only
			string tmpDir = Path.Combine(Environment.GetEnvironmentVariable("GOLDEN_TMP") ?? Path.GetTempPath(), "zbn");
			Directory.CreateDirectory(tmpDir);
			string copy = Path.Combine(tmpDir, "StandardButtons.zbn");
			File.Copy(src, copy, true);
			Buttons = (List<CustomButton>)Tools.Serializer.ObjFromFile(copy);
			if (Buttons == null) throw new Exception("could not decode StandardButtons.zbn");

			StringBuilder js = new StringBuilder();
			js.Append("{\n  \"source\": \"resources/misc/StandardButtons.zbn (gzip + BinaryFormatter List<CustomButton>), decoded by the original Tools.Serializer.ObjFromFile\",\n");
			js.Append("  \"enable_styles\": {\"Always\": 0, \"Connected\": 1, \"Idle\": 3, \"Run\": 4, \"IdleProgram\": 10},\n");
			js.Append("  \"button_types\": {\"Button\": 0, \"TwoStateButton\": 1, \"PushButton\": 2},\n");
			js.Append("  \"buttons\": [\n");
			for (int i = 0; i < Buttons.Count; i++)
			{
				CustomButton b = Buttons[i];
				js.Append("    {\n");
				js.Append("      \"index\": " + i + ",\n");
				js.Append("      \"guid\": " + J(b.guid.ToString()) + ",\n");
				js.Append("      \"caption\": " + J(b.Caption) + ",\n");
				js.Append("      \"tooltip\": " + J(b.ToolTip) + ",\n");
				js.Append("      \"gcode\": " + J(b.GCode) + ",\n");
				js.Append("      \"gcode2\": " + J(b.GCode2) + ",\n");
				js.Append("      \"enable_style\": " + J(b.EnableStyle.ToString()) + ",\n");
				js.Append("      \"enable_style_value\": " + (int)b.EnableStyle + ",\n");
				js.Append("      \"button_type\": " + J(b.ButtonType.ToString()) + ",\n");
				js.Append("      \"button_type_value\": " + (int)b.ButtonType + ",\n");
				if (b.Image == null) js.Append("      \"image_png_base64\": null,\n      \"image_width\": 0,\n      \"image_height\": 0\n");
				else
				{
					using (MemoryStream ms = new MemoryStream())
					{
						b.Image.Save(ms, ImageFormat.Png);
						js.Append("      \"image_png_base64\": \"" + Convert.ToBase64String(ms.ToArray()) + "\",\n");
					}
					js.Append("      \"image_width\": " + b.Image.Width + ",\n      \"image_height\": " + b.Image.Height + "\n");
				}
				js.Append("    }" + (i < Buttons.Count - 1 ? "," : "") + "\n");
			}
			js.Append("  ]\n}\n");
			H.Write("custom_buttons", "standard_buttons.json", js.ToString());
			Console.WriteLine("custom_buttons: " + Buttons.Count + " buttons");

			Eval();
		}

		class Scn
		{
			public string name; public string prog; public GPoint mpos, wco; public decimal jogStep; public int jogSpeed; public string conf; public string[] extra;
		}

		static void Eval()
		{
			H.CopyInput("custom_code", "expressions.txt");
			string progDir = Path.Combine(H.InputsRoot, "gcode_analyze");
			Scn[] scs = {
				new Scn { name = "s0_empty", prog = null, mpos = new GPoint(0, 0, 0), wco = new GPoint(0, 0, 0), jogStep = 10m, jogSpeed = 1000, conf = "default", extra = new string[0] },
				new Scn { name = "s1_square_v11a", prog = "square.nc", mpos = new GPoint(12.5f, -3.25f, 1f), wco = new GPoint(2.5f, 1.5f, 0.25f), jogStep = 5.5m, jogSpeed = 3000, conf = "v11a", extra = new string[0] },
				new Scn { name = "s2_mix_v11b_garbage", prog = "arcs_quadrants.nc", mpos = new GPoint(-5f, -6f, 0.125f), wco = new GPoint(-1f, -2f, 3f), jogStep = 0.1m, jogSpeed = 500, conf = "v11b", extra = new string[] { "$110=abc", "$30=xyz", "$130=12.5mm" } },
				new Scn { name = "s3_unknown_range", prog = "quadrant_unknown.nc", mpos = new GPoint(1f / 3f, 2f / 3f, 0f), wco = new GPoint(0.0005f, 0f, 0f), jogStep = 0.001m, jogSpeed = 1, conf = "v09", extra = new string[] { "$30=2000" } },
				new Scn { name = "s4_negq", prog = "quadrant_III.nc", mpos = new GPoint(100f, 100f, 100f), wco = new GPoint(-100f, 0f, 0f), jogStep = 25m, jogSpeed = 6000, conf = "v11a", extra = new string[] { "$32=0", "$31=0.5" } },
			};

			List<string> exprs = new List<string>();
			foreach (string l in File.ReadAllText(Path.Combine(H.InputsRoot, "custom_code", "expressions.txt"), H.Utf8).Split('\n')) if (l.Length > 0) exprs.Add(l);
			// standard custom buttons (decoded) -> their gcode lines, split like ExecuteCustomCode
			List<string> fromButtons = new List<string>();
			foreach (CustomButton b in Buttons)
				foreach (string code in new string[] { b.GCode, b.GCode2 })
					if (code != null)
						foreach (string line in code.Replace("\r\n", "\n").Split('\n'))
							if (line.Trim().Length > 0) fromButtons.Add(line);

			StringBuilder outb = new StringBuilder();
			outb.Append("# cols: scenario <TAB> source(expr|button) <TAB> input line(escaped) <TAB> GrblCore.EvaluateExpression(line) (escaped)\n");
			StringBuilder sc = new StringBuilder();
			int n = 0;
			ConstructorInfo dummy = null;
			foreach (Scn s in scs)
			{
				H.ResetSettings();
				GrblConfST conf = H.ConfVersion(s.conf) == null ? new GrblConfST() : H.MakeConf(H.ConfVersion(s.conf), H.ConfLines(s.conf));
				foreach (string e in s.extra) conf.AddOrUpdate(e);
				if (s.conf != "default" || s.extra.Length > 0) Settings.SetObject("Grbl Configuration ST", conf);

				// GrblCore needs WinForms stuff in its ctor -> uninitialised object + fields by reflection
				GrblCore core = (GrblCore)FormatterServices.GetUninitializedObject(typeof(GrblCore));
				typeof(GrblCore).GetField("mMPos", H.ALL).SetValue(core, s.mpos);
				typeof(GrblCore).GetField("mWCO", H.ALL).SetValue(core, s.wco);
				core.JogStep = s.jogStep;
				core.JogSpeed = s.jogSpeed;
				GrblFile file = null;
				if (s.prog != null)
				{
					bool ok;
					file = H.LoadNc(Path.Combine(progDir, s.prog), out ok);
				}
				typeof(GrblCore).GetField("file", H.ALL).SetValue(core, file);

				sc.Append("[scenario " + s.name + "]\n");
				sc.Append("program=" + (s.prog ?? "<none>") + "\n");
				sc.Append("MPos=" + H.RF(s.mpos.X) + "," + H.RF(s.mpos.Y) + "," + H.RF(s.mpos.Z) + "\n");
				sc.Append("WCO=" + H.RF(s.wco.X) + "," + H.RF(s.wco.Y) + "," + H.RF(s.wco.Z) + "\n");
				sc.Append("JogStep=" + H.D(s.jogStep) + "\nJogSpeed=" + s.jogSpeed + "\n");
				sc.Append("grbl_conf=" + s.conf + "\n");
				foreach (string l in H.ConfLines(s.conf)) sc.Append("conf.line=" + l + "\n");
				foreach (string l in s.extra) sc.Append("conf.line=" + H.Esc(l) + "\n");
				if (file != null) sc.Append("drawing_range=" + H.RangeStr(file.Range.DrawingRange) + "\n");
				sc.Append("\n");

				foreach (string e in exprs) { outb.Append(s.name + "\texpr\t" + H.Esc(e) + "\t" + H.Esc(Ev(core, e)) + "\n"); n++; }
				foreach (string e in fromButtons) { outb.Append(s.name + "\tbutton\t" + H.Esc(e) + "\t" + H.Esc(Ev(core, e)) + "\n"); n++; }
			}
			H.Write("custom_code", "expected.tsv", outb.ToString());
			H.Write("custom_code", "scenarios.txt", sc.ToString());
			H.WriteConfs("custom_code");

			// IsImmediate / GetImmediate (private, same file)
			string[] imm = { "ctrl-x", "CTRL-X", "Ctrl-X", "?", "!", "~", "0x18", "0X18", "0x85", "0xFF", "0x100", "0x", "0xZZ", "0x-1", "0x 1", "ctrl-y", "", "x", "G0", "M5", "0x1G", "0x7F", "0x00", "?!", "0x0018" };
			StringBuilder ib = new StringBuilder();
			ib.Append("# cols: code(escaped) <TAB> IsImmediate <TAB> GetImmediate (byte value | EXCEPTION:type)\n");
			GrblCore c2 = (GrblCore)FormatterServices.GetUninitializedObject(typeof(GrblCore));
			foreach (string code in imm)
			{
				bool isi = (bool)H.Invoke(c2, "IsImmediate", code);
				string gi;
				try { gi = ((byte)H.Invoke(c2, "GetImmediate", code)).ToString(); } catch (Exception e) { gi = "EXCEPTION: " + H.ExName(e); }
				ib.Append(H.Esc(code) + "\t" + H.B(isi) + "\t" + gi + "\n");
			}
			H.Write("custom_code", "immediate.tsv", ib.ToString());
			Console.WriteLine("custom_code rows: " + n);
		}

		static string Ev(GrblCore core, string line)
		{
			try { return (string)H.Invoke(core, "EvaluateExpression", line); }
			catch (Exception e) { return "EXCEPTION: " + H.ExName(e); }
		}
	}
}

// Shared helpers for the golden harness. Caveman style: small, blunt.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text;
using System.Threading;
using LaserGRBL;

namespace Golden
{
	// All helpers static. One process = one generator run. Settings are global -> reset per scenario.
	public static class H
	{
		public static string GoldenRoot;   // .../tests/golden
		public static string InputsRoot;   // .../tools/golden/harness/inputs
		public static string RepoRoot;     // .../LaserGRBL_linux
		public static readonly Encoding Utf8 = new UTF8Encoding(false);
		public static readonly CultureInfo Inv = CultureInfo.InvariantCulture;
		public const BindingFlags ALL = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static;

		public static readonly Assembly Asm = typeof(GrblFile).Assembly;

		// Escape string to one line: \\ \n \r \t, other control chars as \xHH, lone surrogates as \u{HHHH}.
		public static string Esc(string s)
		{
			if (s == null) return "<null>";
			StringBuilder sb = new StringBuilder();
			for (int i = 0; i < s.Length; i++)
			{
				char c = s[i];
				if (c == '\\') sb.Append("\\\\");
				else if (c == '\n') sb.Append("\\n");
				else if (c == '\r') sb.Append("\\r");
				else if (c == '\t') sb.Append("\\t");
				else if (c < 0x20 || c == 0x7f) sb.Append("\\x" + ((int)c).ToString("X2"));
				else if (char.IsHighSurrogate(c) && i + 1 < s.Length && char.IsLowSurrogate(s[i + 1])) { sb.Append(c); sb.Append(s[i + 1]); i++; }
				else if (char.IsSurrogate(c)) sb.Append("\\u{" + ((int)c).ToString("X4") + "}");
				else sb.Append(c);
			}
			return sb.ToString();
		}

		public static bool Failed = false; // set when a self-check fails; run exits non-zero

		public static string D(decimal d) { return d.ToString(Inv); }
		public static string B(bool b) { return b ? "true" : "false"; }

		// double -> "R" string. Must round-trip, else fall back to G17.
		public static string R(double d)
		{
			if (double.IsNaN(d)) return "NaN";
			if (double.IsPositiveInfinity(d)) return "Infinity";
			if (double.IsNegativeInfinity(d)) return "-Infinity";
			string s = d.ToString("R", Inv);
			double back;
			if (double.TryParse(s, NumberStyles.Float, Inv, out back) && BitConverter.DoubleToInt64Bits(back) == BitConverter.DoubleToInt64Bits(d))
				return s;
			return d.ToString("G17", Inv);
		}

		public static string RF(float d)
		{
			return d.ToString("R", Inv);
		}

		// Hex of the IEEE bits, for bit-exact compare.
		public static string Bits(double d) { return BitConverter.DoubleToInt64Bits(d).ToString("x16"); }

		public static void Write(string area, string rel, string content)
		{
			string path = Path.Combine(GoldenRoot, area, rel);
			Directory.CreateDirectory(Path.GetDirectoryName(path));
			File.WriteAllText(path, content.Replace("\r\n", "\n"), Utf8);
		}

		// Byte-exact write (no newline normalisation).
		public static void WriteRaw(string area, string rel, string content)
		{
			string path = Path.Combine(GoldenRoot, area, rel);
			Directory.CreateDirectory(Path.GetDirectoryName(path));
			File.WriteAllText(path, content, Utf8);
		}

		public static void WriteLines(string area, string rel, IEnumerable<string> lines)
		{
			StringBuilder sb = new StringBuilder();
			foreach (string l in lines) { sb.Append(l); sb.Append('\n'); }
			Write(area, rel, sb.ToString());
		}

		public static void CopyInput(string area, string rel)
		{
			string src = Path.Combine(InputsRoot, area, rel);
			string dst = Path.Combine(GoldenRoot, area, rel);
			Directory.CreateDirectory(Path.GetDirectoryName(dst));
			File.Copy(src, dst, true);
		}

		public static void CopyInput_To(string area, string rel, string toArea)
		{
			string src = Path.Combine(InputsRoot, area, rel);
			string dst = Path.Combine(GoldenRoot, toArea, rel);
			Directory.CreateDirectory(Path.GetDirectoryName(dst));
			File.Copy(src, dst, true);
		}

		// ---- Settings (original static class, persisted to $HOME/.config; HOME is throw-away) ----
		static FieldInfo dicField = typeof(Settings).GetField("dic", ALL);

		public static void ResetSettings()
		{
			System.Collections.IDictionary dic = (System.Collections.IDictionary)dicField.GetValue(null);
			dic.Clear();
		}

		public static void Set(string key, object value)
		{
			Settings.SetObject(key, value);
		}

		// Build config table from "$n=v" lines.
		public static GrblConfST MakeConf(GrblCore.GrblVersionInfo ver, params string[] lines)
		{
			GrblConfST c = new GrblConfST(ver);
			foreach (string l in lines) c.AddOrUpdate(l);
			return c;
		}

		// Named config variants used everywhere. "default" = no version info (all defaults: MaxRateX 4000, MaxPWM 1000).
		public static readonly string[] ConfNames = { "default", "v08", "v09", "v11a", "v11b" };
		public static string[] ConfLines(string name)
		{
			switch (name)
			{
				case "v08": return new string[] { "$4=1500", "$5=1200", "$0=10", "$1=10", "$17=0" };
				case "v09": return new string[] { "$110=2000", "$111=1800", "$130=250", "$131=180", "$100=200", "$101=200", "$22=1" };
				case "v11a": return new string[] { "$110=3000", "$111=2500", "$130=400", "$131=300", "$30=255", "$31=0", "$32=1", "$100=250", "$101=250", "$120=100", "$121=200", "$22=1", "$20=1" };
				case "v11b": return new string[] { "$110=12000", "$111=9000", "$130=800", "$131=600", "$30=1000", "$31=10", "$32=0" };
				default: return new string[0];
			}
		}
		public static GrblCore.GrblVersionInfo ConfVersion(string name)
		{
			switch (name)
			{
				case "v08": return new GrblCore.GrblVersionInfo(0, 8);
				case "v09": return new GrblCore.GrblVersionInfo(0, 9);
				case "v11a": return new GrblCore.GrblVersionInfo(1, 1);
				case "v11b": return new GrblCore.GrblVersionInfo(1, 1, 'f');
				default: return null;
			}
		}
		// Writes <area>/confs.txt describing every named GrblConfST variant (version + $n=v lines + MaxRateX/MaxPWM).
		public static void WriteConfs(string area)
		{
			StringBuilder confs = new StringBuilder();
			foreach (string cn in ConfNames)
			{
				GrblCore.GrblVersionInfo v = ConfVersion(cn);
				confs.Append("[conf " + cn + "]\n");
				confs.Append("version=" + (v == null ? "none" : v.ToString()) + "\n");
				foreach (string l in ConfLines(cn)) confs.Append("line=" + l + "\n");
				GrblConfST c = cn == "default" ? new GrblConfST() : MakeConf(v, ConfLines(cn));
				confs.Append("MaxRateX=" + D(c.MaxRateX) + "\nMaxPWM=" + D(c.MaxPWM) + "\n\n");
			}
			Write(area, "confs.txt", confs.ToString());
		}

		public static void ApplyConf(string name)
		{
			if (name == "default") return; // nothing stored -> GrblConfST() with no version
			Settings.SetObject("Grbl Configuration ST", MakeConf(ConfVersion(name), ConfLines(name)));
		}

		// ---- File loading helpers ----
		public static void WaitLoaded(GrblFile f)
		{
			int n = 0;
			while (f.CheckInUse(false))
			{
				Thread.Sleep(2);
				if (++n > 30000) throw new Exception("load timeout");
			}
		}

		// Load .nc through the original threaded LoadFile. ok=false if the load thread died (exception).
		public static GrblFile LoadNc(string path, out bool ok)
		{
			GrblFile f = new GrblFile();
			bool done = false;
			f.OnFileLoaded += delegate (long el, string fn) { done = true; };
			f.LoadFile(path, false);
			WaitLoaded(f);
			ok = done;
			if (!ok) { Console.Error.WriteLine("LOAD FAILED (thread died): " + path); Failed = true; }
			return f;
		}

		public static object Invoke(object target, string name, params object[] args)
		{
			Type t = target as Type ?? target.GetType();
			object inst = target is Type ? null : target;
			MethodInfo[] ms = t.GetMethods(ALL);
			foreach (MethodInfo m in ms)
			{
				if (m.Name != name) continue;
				ParameterInfo[] ps = m.GetParameters();
				if (ps.Length != args.Length) continue;
				try { return m.Invoke(inst, args); }
				catch (TargetInvocationException e) { throw e.InnerException; }
			}
			throw new MissingMethodException(t.FullName + "." + name);
		}

		public static string ExName(Exception e)
		{
			while (e is TargetInvocationException && e.InnerException != null) e = e.InnerException;
			return e.GetType().FullName;
		}

		// Plain "key=value" header writer.
		public class KV
		{
			public StringBuilder sb = new StringBuilder();
			public void Add(string k, string v) { sb.Append(k); sb.Append('='); sb.Append(v); sb.Append('\n'); }
			public void Add(string k, object v) { Add(k, Convert.ToString(v, Inv)); }
			public void AddEsc(string k, string v) { Add(k, Esc(v)); }
			public override string ToString() { return sb.ToString(); }
		}

		public static string RangeStr(ProgramRange.XYRange r)
		{
			return string.Format("xmin={0} xmax={1} ymin={2} ymax={3} valid={4}", D(r.X.Min), D(r.X.Max), D(r.Y.Min), D(r.Y.Max), B(r.ValidRange));
		}
	}
}

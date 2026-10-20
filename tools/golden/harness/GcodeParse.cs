// gcode_parse: GrblCommand parsing oracle.
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Text;
using LaserGRBL;

namespace Golden
{
	public static class GcodeParse
	{
		public static void Run()
		{
			H.CopyInput("gcode_parse", "corpus.txt");
			string text = File.ReadAllText(Path.Combine(H.InputsRoot, "gcode_parse", "corpus.txt"), H.Utf8);
			List<string> lines = new List<string>(text.Split('\n'));
			if (lines.Count > 0 && lines[lines.Count - 1] == "") lines.RemoveAt(lines.Count - 1); // trailing LF only

			StringBuilder sb = new StringBuilder();
			for (int i = 0; i < lines.Count; i++)
			{
				string line = lines[i];
				sb.Append("[case " + i + "]\n");
				H.KV kv = new H.KV();
				kv.AddEsc("in", line);
				Dump(kv, "", delegate { return new GrblCommand(line); });
				kv.AddEsc("preserve.Command", new GrblCommand(line, 0, true).Command);

				// JogCommand only for "$J=" lines (as MainForm/JogForm do): ctor does Substring(3).
				if (line.Length >= 3 && line.Substring(0, 3).ToUpperInvariant() == "$J=")
				{
					kv.Add("jog", "yes");
					Dump(kv, "jog.", delegate { return new JogCommand(line); });
				}
				sb.Append(kv.ToString());
				sb.Append('\n');
			}
			H.Write("gcode_parse", "expected.txt", sb.ToString());
			Console.WriteLine("gcode_parse cases: " + lines.Count);
		}

		delegate GrblCommand Maker();

		// Copy of GrblCommand.BuildHelper control flow. Returns exception type name thrown (swallowed by original) or "none".
		static string ReplicaBuild(string mLine, out string result)
		{
			result = mLine;
			if (mLine.StartsWith("$")) return "none";
			Dictionary<char, decimal> seen = new Dictionary<char, decimal>();
			char cmd = '\0';
			string num = "";
			bool comment = false, oldspace = false;
			StringBuilder sb = new StringBuilder();
			try
			{
				foreach (char c in mLine)
				{
					if (c == ';') break;
					if (c == '(') comment = true;
					bool space = c == ' ';
					if (!comment)
					{
						if (space && !oldspace) sb.Append(' ');
						else if (!space) sb.Append(c);
					}
					oldspace = space;
					if (!comment)
					{
						if (Char.IsLetter(c))
						{
							if (cmd != '\0') { seen.Add(cmd, Decimal.Parse(num, System.Globalization.NumberFormatInfo.InvariantInfo)); }
							cmd = c; num = "";
						}
						else if (Char.IsNumber(c) || c == '.' || c == '-') num += c;
					}
					if (c == ')') comment = false;
				}
				result = sb.ToString();
				if (cmd != '\0') seen.Add(cmd, Decimal.Parse(num, System.Globalization.NumberFormatInfo.InvariantInfo));
			}
			catch (Exception e) { result = mLine; return e.GetType().FullName; }
			return "none";
		}

		static void Dump(H.KV kv, string p, Maker mk)
		{
			GrblCommand c = mk();
			kv.AddEsc(p + "pre.Command", c.Command);
			kv.AddEsc(p + "pre.SerialData", c.SerialData);
			kv.Add(p + "pre.IsEmpty", H.B(c.IsEmpty));
			kv.Add(p + "pre.IsGrblCommand", H.B(c.IsGrblCommand));
			kv.Add(p + "pre.IsWriteEEPROM", H.B(c.IsWriteEEPROM));
			kv.Add(p + "pre.JustBuilt", H.B(c.JustBuilt));

			string preCmd = c.Command;
			c.BuildHelper();
			// Mono has no FirstChanceException, so replay the parse loop to learn which exception BuildHelper swallowed.
			string repSb;
			string repEx = ReplicaBuild(preCmd, out repSb);
			kv.Add(p + "build.swallowed_exception", repEx);

			// replica check: no throw -> mLine == rebuilt string; throw -> mLine untouched
			string expectPost = (preCmd.StartsWith("$") ? preCmd : (repEx == "none" ? repSb : preCmd));
			if (expectPost != c.Command) { Console.Error.WriteLine("REPLICA MISMATCH for [" + H.Esc(preCmd) + "]"); H.Failed = true; }
			kv.AddEsc(p + "post.Command", c.Command);
			kv.AddEsc(p + "post.SerialData", c.SerialData);
			kv.Add(p + "post.IsEmpty", H.B(c.IsEmpty));
			kv.Add(p + "post.IsGrblCommand", H.B(c.IsGrblCommand));
			kv.Add(p + "post.IsWriteEEPROM", H.B(c.IsWriteEEPROM));
			kv.Add(p + "post.JustBuilt", H.B(c.JustBuilt));

			// elements in dictionary enumeration order (= insertion order, no removals)
			FieldInfo fi = typeof(GrblCommand).GetField("mHelper", H.ALL);
			System.Collections.IDictionary d = (System.Collections.IDictionary)fi.GetValue(c);
			List<string> els = new List<string>();
			foreach (System.Collections.DictionaryEntry de in d)
			{
				GrblCommand.Element e = (GrblCommand.Element)de.Value;
				els.Add(e.Command.ToString() + ":" + H.D(e.Number));
			}
			kv.AddEsc(p + "elements", els.Count == 0 ? "<none>" : string.Join("|", els.ToArray()));
			kv.AddEsc(p + "ToString", c.ToString());

			kv.Add(p + "IsMovement", H.B(c.IsMovement));
			kv.Add(p + "IsLinearMovement", H.B(c.IsLinearMovement));
			kv.Add(p + "IsArcMovement", H.B(c.IsArcMovement));
			kv.Add(p + "IsPause", H.B(c.IsPause));
			kv.Add(p + "IsLaserON", H.B(c.IsLaserON));
			kv.Add(p + "IsLaserOFF", H.B(c.IsLaserOFF));
			kv.Add(p + "IsSetWCO", H.B(c.IsSetWCO));
			kv.Add(p + "IsAbsoluteCoord", H.B(c.IsAbsoluteCoord));
			kv.Add(p + "IsRelativeCoord", H.B(c.IsRelativeCoord));
			kv.Add(p + "IsM3", H.B(c.IsM3));
			kv.Add(p + "IsM4", H.B(c.IsM4));
			kv.Add(p + "IsM5", H.B(c.IsM5));
			kv.Add(p + "IsCW(false)", H.B(c.IsCW(false)));
			kv.Add(p + "IsCW(true)", H.B(c.IsCW(true)));
		}
	}
}

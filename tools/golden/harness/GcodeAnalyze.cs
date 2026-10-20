// gcode_analyze + state_builder oracle.
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using LaserGRBL;

namespace Golden
{
	public static class GcodeAnalyze
	{
		static string[] Programs()
		{
			string[] f = Directory.GetFiles(Path.Combine(H.InputsRoot, "gcode_analyze"), "*.nc");
			Array.Sort(f, StringComparer.Ordinal);
			return f;
		}

		public static void Run()
		{
			string dirA = Path.Combine(H.GoldenRoot, "gcode_analyze");
			string dirS = Path.Combine(H.GoldenRoot, "state_builder");
			if (Directory.Exists(dirA)) Directory.Delete(dirA, true);
			if (Directory.Exists(dirS)) Directory.Delete(dirS, true);

			H.WriteConfs("gcode_analyze");
			H.WriteConfs("state_builder");

			int nA = 0, nS = 0;
			foreach (string prog in Programs())
			{
				string name = Path.GetFileName(prog);
				H.CopyInput("gcode_analyze", name);
				bool big = name == "raster_like.nc";

				foreach (string cn in H.ConfNames)
				{
					if (big && (cn == "v08" || cn == "v11b")) continue;
					H.ResetSettings();
					H.ApplyConf(cn);
					H.Set("Support Hardware PWM", true);
					bool ok;
					GrblFile f = H.LoadNc(prog, out ok);
					StringBuilder sb = new StringBuilder();
					sb.Append("program=" + name + "\n");
					sb.Append("conf=" + cn + "\n");
					sb.Append("load_ok=" + H.B(ok) + "\n");
					sb.Append("count=" + f.Count + "\n");
					sb.Append("estimated_ticks=" + f.EstimatedTime.Ticks + "\n");
					sb.Append("drawing_range=" + H.RangeStr(f.Range.DrawingRange) + "\n");
					sb.Append("moving_range=" + H.RangeStr(f.Range.MovingRange) + "\n");
					sb.Append("spindle_range=smin=" + H.D(f.Range.SpindleRange.S.Min) + " smax=" + H.D(f.Range.SpindleRange.S.Max) + " valid=" + H.B(f.Range.SpindleRange.ValidRange) + "\n");
					sb.Append("quadrant=" + f.Quadrant + "\n");
					sb.Append("---\n");
					for (int i = 0; i < f.Count; i++)
						sb.Append(i + "\t" + f[i].TimeOffset.Ticks + "\t" + H.Esc(f[i].Command) + "\n");
					H.Write("gcode_analyze", name.Replace(".nc", "") + "." + cn + ".txt", sb.ToString());
					nA++;
				}

				foreach (bool pwm in new bool[] { true, false })
				{
					if (big && !pwm) continue;
					H.ResetSettings();
					H.Set("Support Hardware PWM", pwm);
					H.Write("state_builder", name.Replace(".nc", "") + ".pwm" + (pwm ? "1" : "0") + ".txt", Trace(prog, name, pwm));
					nS++;
				}
			}
			Console.WriteLine("gcode_analyze files: " + nA + ", state_builder files: " + nS);
		}

		static string Q(H.KV kv) { return kv.ToString(); }

		static string Trace(string path, string name, bool pwm)
		{
			// same list building as GrblFile.LoadFile
			List<GrblCommand> list = new List<GrblCommand>();
			foreach (string raw in File.ReadAllText(path, H.Utf8).Split('\n'))
			{
				string line = raw.Replace("\r", "").Trim();
				if (line.Length > 0)
				{
					GrblCommand cmd = new GrblCommand(line);
					if (!cmd.IsEmpty) list.Add(cmd);
				}
			}

			StringBuilder sb = new StringBuilder();
			sb.Append("program=" + name + "\n");
			sb.Append("support_pwm=" + H.B(pwm) + "\n");
			sb.Append("conf=default\n");
			sb.Append("count=" + list.Count + "\n---\n");

			GrblCommand.StatePositionBuilder spb = new GrblCommand.StatePositionBuilder();
			GrblConfST conf = GrblCore.Configuration;
			object prevArc = null;
			for (int i = 0; i < list.Count; i++)
			{
				GrblCommand cmd = list[i];
				TimeSpan delay = spb.AnalyzeCommand(cmd, true, conf);
				List<string> f = new List<string>();
				f.Add("i=" + i);
				f.Add("cmd=" + H.Esc(cmd.Command));
				f.Add("delay=" + delay.Ticks);
				f.Add("X=" + H.D(spb.X.Number) + "/" + H.D(spb.X.Previous));
				f.Add("Y=" + H.D(spb.Y.Number) + "/" + H.D(spb.Y.Previous));
				f.Add("Z=" + H.D(spb.Z.Number) + "/" + H.D(spb.Z.Previous));
				f.Add("F=" + H.D(spb.F.Number));
				f.Add("S=" + H.D(spb.S.Number));
				f.Add("burn=" + H.B(spb.LaserBurning));
				f.Add("G0G1=" + H.B(spb.G0G1));
				f.Add("G2G3=" + H.B(spb.G2G3));
				f.Add("true_move=" + H.B((bool)H.Invoke(spb, "TrueMovement")));
				f.Add("motion=" + spb.MotionMode.ToString());
				f.Add("abs=" + H.B(spb.ABS));
				f.Add("m3m4=" + H.B(spb.M3M4));
				f.Add("wco=" + H.D(spb.WcoX) + "," + H.D(spb.WcoY) + "," + H.D(spb.WcoZ));
				f.Add("settled=" + H.Esc(new GrblCommand(spb.GetSettledModals()).Command));
				GrblCommand.G2G3Helper a = spb.LastArcHelperResult;
				if (a == null) f.Add("arc=none");
				else
				{
					f.Add("arc=" + (object.ReferenceEquals(a, prevArc) ? "stale" : "fresh"));
					f.Add("arc.cx=" + H.R(a.CenterX));
					f.Add("arc.cy=" + H.R(a.CenterY));
					f.Add("arc.ray=" + H.R(a.Ray));
					f.Add("arc.start=" + H.R(a.StartAngle));
					f.Add("arc.end=" + H.R(a.EndAngle));
					f.Add("arc.width=" + H.R(a.AngularWidth));
					f.Add("arc.cw=" + H.B(a.CW));
					f.Add("arc.len=" + H.R(a.Lenght));
					f.Add("arc.abslen=" + H.R(a.AbsLenght));
					f.Add("arc.bbox=" + H.R(a.BBox.X) + "," + H.R(a.BBox.Y) + "," + H.R(a.BBox.Width) + "," + H.R(a.BBox.Height));
					f.Add("arc.bits=" + H.Bits(a.CenterX) + "," + H.Bits(a.CenterY) + "," + H.Bits(a.Ray) + "," + H.Bits(a.StartAngle) + "," + H.Bits(a.EndAngle) + "," + H.Bits(a.AngularWidth) + "," + H.Bits(a.BBox.X) + "," + H.Bits(a.BBox.Y) + "," + H.Bits(a.BBox.Width) + "," + H.Bits(a.BBox.Height));
				}
				prevArc = a;
				sb.Append(string.Join("\t", f.ToArray()));
				sb.Append('\n');
			}
			return sb.ToString();
		}
	}
}

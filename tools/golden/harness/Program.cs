// Golden harness entry. Usage: Golden.exe <repoRoot> [area ...]
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Threading;

namespace Golden
{
	public static class Program
	{
		public static int Main(string[] args)
		{
			// Invariant culture everywhere: original code mostly passes InvariantCulture, but not always.
			Thread.CurrentThread.CurrentCulture = CultureInfo.InvariantCulture;
			Thread.CurrentThread.CurrentUICulture = CultureInfo.InvariantCulture;
			CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
			CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.InvariantCulture;

			if (args.Length < 1) { Console.Error.WriteLine("usage: Golden.exe <repoRoot> [areas]"); return 2; }
			H.RepoRoot = Path.GetFullPath(args[0]);
			H.GoldenRoot = Path.Combine(H.RepoRoot, "tests", "golden");
			H.InputsRoot = Path.Combine(H.RepoRoot, "tools", "golden", "harness", "inputs");

			Dictionary<string, Action> areas = new Dictionary<string, Action>();
			areas["gcode_parse"] = GcodeParse.Run;
			areas["gcode_analyze"] = GcodeAnalyze.Run;      // also writes state_builder
			areas["l2l"] = L2L.Run;
			areas["potrace"] = PotraceGold.Run;             // also potrace_curves + potrace_rasterfill
			areas["svg"] = SvgGold.Run;
			areas["generators"] = Generators.Run;
			areas["config"] = ConfigGold.Run;
			areas["timespan_fmt"] = TimespanGold.Run;
			areas["custom_code"] = CustomCode.Run;          // also custom_buttons

			List<string> want = new List<string>();
			for (int i = 1; i < args.Length; i++) want.Add(args[i]);
			if (want.Count == 0) want.AddRange(areas.Keys);

			int rc = 0;
			foreach (string a in want)
			{
				if (!areas.ContainsKey(a)) { Console.Error.WriteLine("unknown area " + a); return 2; }
				Console.WriteLine("== " + a);
				try
				{
					H.ResetSettings();
					areas[a]();
				}
				catch (Exception e) { Console.Error.WriteLine("AREA FAILED " + a + ": " + e); rc = 1; }
			}
			// Settings timer may still be pending; exit hard so nothing persists beyond the throw-away HOME.
			Console.Out.Flush();
			Environment.Exit(H.Failed ? 1 : rc);
			return rc;
		}
	}
}

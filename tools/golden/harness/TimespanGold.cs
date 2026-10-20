// timespan_fmt: Tools.Utils.TimeSpanToString / HumanReadableDateDiff oracle.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using Tools;

namespace Golden
{
	public static class TimespanGold
	{
		static TimeSpan TS(int d, int h, int m, int s, int ms) { return new TimeSpan(d, h, m, s, ms); }

		public static void Run()
		{
			List<TimeSpan> spans = new List<TimeSpan>();
			spans.Add(TimeSpan.Zero);
			spans.Add(TimeSpan.FromTicks(1)); spans.Add(TimeSpan.FromTicks(9999)); spans.Add(TimeSpan.FromTicks(12345)); spans.Add(TimeSpan.FromTicks(10000));
			spans.Add(TS(0, 0, 0, 0, 1)); spans.Add(TS(0, 0, 0, 0, 999)); spans.Add(TS(0, 0, 0, 1, 0)); spans.Add(TS(0, 0, 0, 1, 1));
			spans.Add(TS(0, 0, 0, 59, 999)); spans.Add(TS(0, 0, 1, 0, 0)); spans.Add(TS(0, 0, 1, 1, 1)); spans.Add(TS(0, 0, 59, 59, 999));
			spans.Add(TS(0, 1, 0, 0, 0)); spans.Add(TS(0, 1, 1, 1, 0)); spans.Add(TS(0, 2, 0, 30, 0)); spans.Add(TS(0, 23, 59, 59, 999));
			spans.Add(TS(1, 0, 0, 0, 0)); spans.Add(TS(1, 2, 3, 4, 5)); spans.Add(TS(2, 0, 0, 1, 0)); spans.Add(TS(10, 10, 10, 10, 10));
			spans.Add(TS(27, 23, 59, 59, 999)); spans.Add(TS(6, 0, 0, 0, 0));
			// fractional-second spans like the estimator produces
			spans.Add(TimeSpan.FromMinutes(0.5)); spans.Add(TimeSpan.FromMinutes(1.0 / 3)); spans.Add(TimeSpan.FromSeconds(3661.5)); spans.Add(TimeSpan.FromMilliseconds(500.4));
			spans.Add(TimeSpan.FromTicks(12345678901L)); spans.Add(TimeSpan.FromTicks(600000000L + 5000)); spans.Add(TimeSpan.FromTicks(36000000000L - 1));
			List<TimeSpan> all = new List<TimeSpan>(spans);
			foreach (TimeSpan t in spans) if (t != TimeSpan.Zero) all.Add(t.Negate());
			all.Add(TimeSpan.MaxValue); all.Add(TimeSpan.MinValue);
			all.Add(TimeSpan.MaxValue - TimeSpan.FromTicks(1)); all.Add(TimeSpan.MinValue + TimeSpan.FromTicks(1));

			StringBuilder sb = new StringBuilder();
			sb.Append("# Tools.Utils.TimeSpanToString(Span=new TimeSpan(ticks), Precision, MaxFallBackPrecision, Separator, WriteSuffix)\n");
			sb.Append("# cols: ticks <TAB> precision(int) <TAB> fallback(int) <TAB> separator(escaped) <TAB> writeSuffix <TAB> result(escaped) | EXCEPTION:type\n");
			sb.Append("# spans below 28 days only depend on the value; spans >= 28 days would depend on DateTime.Now's calendar (see hrdd.tsv)\n");
			int n = 0;
			foreach (TimeSpan t in all)
			{
				if (t > TimeSpan.FromDays(28) && t < TimeSpan.MaxValue - TimeSpan.FromTicks(1)) continue;
				if (t < TimeSpan.FromDays(-28) && t > TimeSpan.MinValue + TimeSpan.FromTicks(1)) continue;
				for (int p = 0; p <= 6; p++)
					for (int f = 0; f <= 6; f++)
						foreach (bool sfx in new bool[] { true, false })
						{
							// huge spans: only a few combos (Max/Min short-circuit)
							if ((t >= TimeSpan.MaxValue - TimeSpan.FromTicks(1) || t <= TimeSpan.MinValue + TimeSpan.FromTicks(1)) && !(p == 5 && f == 5)) continue;
							Row(sb, t, (Utils.TimePrecision)p, (Utils.TimePrecision)f, " ,", sfx); n++;
						}
				// other separators only for the combos the app uses
				int[][] app = { new int[] { 5, 5 }, new int[] { 4, 5 } };
				foreach (int[] pf in app)
					foreach (string sep in new string[] { ",", " - ", "" })
						foreach (bool sfx in new bool[] { true, false })
						{ Row(sb, t, (Utils.TimePrecision)pf[0], (Utils.TimePrecision)pf[1], sep, sfx); n++; }
			}
			H.Write("timespan_fmt", "timespan_to_string.tsv", sb.ToString());

			// HumanReadableDateDiff with fixed dates
			StringBuilder hb = new StringBuilder();
			hb.Append("# Tools.Utils.HumanReadableDateDiff(MainDate, OtherDate, Precision, MaxFallBackPrecision, Separator=\" ,\", WriteSuffix)\n");
			hb.Append("# cols: main(yyyy-MM-ddTHH:mm:ss.fffffff, Kind=Unspecified) <TAB> other <TAB> precision <TAB> fallback <TAB> separator(escaped) <TAB> writeSuffix <TAB> result(escaped)\n");
			string[] anchors = { "2024-01-31T12:00:00.0000000", "2024-02-29T00:00:00.0000000", "2023-03-31T08:30:15.0000000", "2024-12-31T23:59:59.9990000", "2023-02-28T12:00:00.0000000", "2020-02-29T06:00:00.0000000" };
			double[] offsetDays = { 0, 0.5, 27.99, 28, 29, 30, 31, 32, 59, 60, 61, 90.25, 365, 366, 400, 730.5, 800, 1461, 3650 };
			int[][] pfs = { new int[] { 0, 0 }, new int[] { 0, 6 }, new int[] { 1, 6 }, new int[] { 2, 6 }, new int[] { 3, 6 }, new int[] { 5, 5 }, new int[] { 4, 5 }, new int[] { 6, 6 } };
			int hn = 0;
			foreach (string a in anchors)
			{
				DateTime A = DateTime.ParseExact(a, "yyyy-MM-ddTHH:mm:ss.fffffff", CultureInfo.InvariantCulture);
				foreach (double od in offsetDays)
					foreach (int dir in new int[] { 1, -1 })
					{
						DateTime B = A.AddTicks((long)(od * TimeSpan.TicksPerDay) * dir);
						foreach (int[] pf in pfs)
							foreach (bool sfx in new bool[] { true, false })
							{
								string res;
								try { res = Utils.HumanReadableDateDiff(A, B, (Utils.TimePrecision)pf[0], (Utils.TimePrecision)pf[1], " ,", sfx); }
								catch (Exception e) { res = "EXCEPTION: " + H.ExName(e); }
								hb.Append(A.ToString("yyyy-MM-ddTHH:mm:ss.fffffff", CultureInfo.InvariantCulture) + "\t" + B.ToString("yyyy-MM-ddTHH:mm:ss.fffffff", CultureInfo.InvariantCulture) + "\t" + pf[0] + "\t" + pf[1] + "\t" + H.Esc(" ,") + "\t" + H.B(sfx) + "\t" + H.Esc(res) + "\n");
								hn++;
							}
					}
			}
			H.Write("timespan_fmt", "hrdd.tsv", hb.ToString());
			Console.WriteLine("timespan_fmt rows: " + n + " + " + hn);
		}

		static void Row(StringBuilder sb, TimeSpan t, Utils.TimePrecision p, Utils.TimePrecision f, string sep, bool sfx)
		{
			string res;
			try { res = H.Esc(Utils.TimeSpanToString(t, p, f, sep, sfx)); }
			catch (Exception e) { res = "EXCEPTION: " + H.ExName(e); }
			sb.Append(t.Ticks + "\t" + (int)p + "\t" + (int)f + "\t" + H.Esc(sep) + "\t" + H.B(sfx) + "\t" + res + "\n");
		}
	}
}

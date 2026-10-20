// config: GrblConfST parsing + derived properties oracle.
using System;
using System.Collections.Generic;
using System.Text;
using LaserGRBL;

namespace Golden
{
	public static class ConfigGold
	{
		static string[] Single = {
			"$110=6000", "$111=5000.5", "$110 = 3000", "$110=  3000  ", "$110=abc", "$110=", "$110=1e3", "$110=1,5", "$110=-5", "$110=0", "$110=99999999",
			"$110=2000000", "$110=2000001", "$110=12abc34", "$110=abc12.5def", "$110=.5", "$110=0.5", "$110=5.", "$110=+5", "$110=1 000", "$110=١٢٣",
			"$4=1500", "$5=250.25", "$0=10", "$1=25", "$17=1", "$17=0", "$17=2", "$20=0", "$20=1", "$20=2", "$20=-1", "$20=0.5",
			"$20=0 (soft limits, bool)", "$20=1 (soft limits, bool)", "$20 = 0 (soft limits, bool)", "$20=0.5 (note)", "$22=1", "$22=0", "$22=5",
			"$30=255", "$30=0", "$30=-3", "$30=1000.7", "$31=100", "$31=-1", "$31=3000000", "$32=1", "$32=0", "$32=2", "$32=0.2",
			"$100=80.000", "$100=0", "$100=20000", "$101=-1", "$101=250", "$120=500", "$121=700", "$120=0", "$121=3000000",
			"$130=800", "$131=600", "$130=0", "$130=3000000", "$131=abc",
			"$74=MySSID", "$75=secret pw", "$75=", "$305=2323", "$305=", "$305=abc", "$23=1", "$33=1", "$13=1",
			"$110=6000 (x-rate, mm/min)", "$110=6000(x)", "$110=6000.50 (note)", "$110=abc (note)", "$100=250.000 (steps)", "$110=6000.50.1 (x)", "$110=-6000 (neg)",
			"$110 =6000", "$110= 6000", "  $110=6000", "$110=6000 ", "$110=6000\r", "$110=6000\t", "$ 110=6000", "$110 6000", "110=6000", "$110:6000",
			"garbage", "$", "$=", "$a=1", "=1", "$1", "", "   ", "$$", "$H", "$G", "$#",
			"$99999999999=1", "$-1=2", "$01=5", "$1==2", "$1=a=b", "$1=2=3", "$00110=6000", "$110=6000\n", "ok", "[GC:G0 G54]", "$I", "$RST=*",
			"$110=6000 (a) (b)", "$110=(x)", "$110=6000 (", "$110=6000)", "$110=1e", "$110=∞", "$110=NaN",
		};

		// multi-line scenarios (applied in order)
		static string[][] Scenarios = {
			new string[] { "$0=10", "$1=25", "$2=0", "$3=0", "$4=0", "$5=0", "$6=0", "$10=1", "$11=0.010", "$12=0.002", "$13=0", "$20=0", "$21=0", "$22=1", "$23=3", "$24=25.000", "$25=500.000", "$26=250", "$27=1.000", "$30=1000", "$31=0", "$32=1", "$100=80.000", "$101=80.000", "$102=250.000", "$110=6000.000", "$111=6000.000", "$112=500.000", "$120=200.000", "$121=200.000", "$122=10.000", "$130=300.000", "$131=200.000", "$132=200.000" },
			new string[] { "$0=10", "$1=25", "$2=0", "$3=0", "$4=0", "$5=0", "$6=0", "$10=3", "$11=0.010", "$12=0.002", "$13=0", "$20=0", "$21=0", "$22=0", "$23=0", "$24=25.000", "$25=500.000", "$26=250", "$27=1.000", "$100=250.000", "$101=250.000", "$102=250.000", "$110=500.000", "$111=500.000", "$112=500.000", "$120=10.000", "$121=10.000", "$122=10.000", "$130=200.000", "$131=200.000", "$132=200.000" },
			new string[] { "$100=250", "$100=300", "$110=1000", "$110=2000", "$110=abc", "$110=3000" },
			new string[] { "$20=0 (soft limits, bool)", "$21=0 (hard limits, bool)", "$22=1 (homing cycle, bool)", "$30=1000 (max spindle speed, RPM)", "$32=1 (laser mode, bool)", "$110=8000.000 (x max rate, mm/min)", "$111=8000.000 (y max rate, mm/min)", "$130=400.000 (x max travel, mm)", "$131=400.000 (y max travel, mm)" },
			new string[] { "$110=7000", "garbage", "$111=7000", "", "$130=450", "$131=350", "$305=2323", "$74=ssid", "$75=pw" },
			new string[] { "$32=1", "$30=255", "$31=5", "$110=15000", "$111=15000", "$22=0", "$20=1", "$100=100", "$101=100", "$120=1000", "$121=3000" },
		};

		static Dictionary<string, GrblCore.GrblVersionInfo> Versions()
		{
			Dictionary<string, GrblCore.GrblVersionInfo> d = new Dictionary<string, GrblCore.GrblVersionInfo>();
			d["none"] = null;
			d["0.8"] = new GrblCore.GrblVersionInfo(0, 8);
			d["0.9"] = new GrblCore.GrblVersionInfo(0, 9);
			d["0.9j"] = new GrblCore.GrblVersionInfo(0, 9, 'j');
			d["1.0"] = new GrblCore.GrblVersionInfo(1, 0);
			d["1.1"] = new GrblCore.GrblVersionInfo(1, 1);
			d["1.1f"] = new GrblCore.GrblVersionInfo(1, 1, 'f');
			d["1.2"] = new GrblCore.GrblVersionInfo(1, 2);
			d["ortur1.1"] = new GrblCore.GrblVersionInfo(1, 1, 'f', "Ortur Laser Master 2", "170", false);
			return d;
		}
		static string[] VNames = { "none", "0.8", "0.9", "0.9j", "1.0", "1.1", "1.1f", "1.2", "ortur1.1" };

		public static void Run()
		{
			Dictionary<string, GrblCore.GrblVersionInfo> vers = Versions();
			StringBuilder sb = new StringBuilder();
			int n = 0;
			foreach (string line in Single) { Case(sb, vers, n++, new string[] { line }); }
			foreach (string[] sc in Scenarios) { Case(sb, vers, n++, sc); }
			H.Write("config", "cases.txt", sb.ToString());

			// version info helper facts
			StringBuilder vs = new StringBuilder();
			foreach (string vn in VNames)
			{
				GrblCore.GrblVersionInfo v = vers[vn];
				vs.Append("[version " + vn + "]\n");
				if (v == null) { vs.Append("null\n\n"); continue; }
				vs.Append("ToString=" + v + "\nMajor=" + v.Major + "\nMinor=" + v.Minor + "\nIsOrtur=" + H.B(v.IsOrtur) + "\nIsLonger=" + H.B(v.IsLonger) + "\nIsHAL=" + H.B(v.IsHAL) + "\nOrturFWVersionNumber=" + v.OrturFWVersionNumber + "\n");
				foreach (string vn2 in VNames)
				{
					GrblCore.GrblVersionInfo w = vers[vn2];
					if (w == null) continue;
					vs.Append("cmp(" + vn2 + ")=" + v.CompareTo(w) + " ge=" + H.B(v >= w) + " eq=" + H.B(v == w) + "\n");
				}
				vs.Append("\n");
			}
			H.Write("config", "versions.txt", vs.ToString());
			Console.WriteLine("config cases: " + n);
		}

		static void Case(StringBuilder sb, Dictionary<string, GrblCore.GrblVersionInfo> vers, int n, string[] lines)
		{
			sb.Append("[case " + n + "]\n");
			for (int i = 0; i < lines.Length; i++) sb.Append("line." + i + "=" + H.Esc(lines[i]) + "\n");
			for (int i = 0; i < lines.Length; i++) sb.Append("IsSetConf." + i + "=" + H.B(GrblConfST.IsSetConf(lines[i])) + "\n");
			foreach (string vn in VNames)
			{
				GrblConfST c = new GrblConfST(vers[vn]);
				string p = "v[" + vn + "].";
				foreach (string l in lines)
				{
					try { c.AddOrUpdate(l); }
					catch (Exception e) { sb.Append(p + "AddOrUpdate.exception=" + H.ExName(e) + "\n"); }
				}
				sb.Append(p + "Count=" + c.Count + "\n");
				List<string> kv = new List<string>();
				foreach (KeyValuePair<int, string> k in c) kv.Add(k.Key + "=" + H.Esc(k.Value));
				sb.Append(p + "data=" + (kv.Count == 0 ? "<empty>" : string.Join("|", kv.ToArray())) + "\n");
				sb.Append(p + "ExpectedCount=" + c.ExpectedCount + "\n");
				sb.Append(p + "HomingEnabled=" + H.B(c.HomingEnabled) + "\n");
				sb.Append(p + "MaxRateX=" + H.D(c.MaxRateX) + "\n");
				sb.Append(p + "MaxRateY=" + H.D(c.MaxRateY) + "\n");
				sb.Append(p + "LaserMode=" + H.B(c.LaserMode) + "\n");
				sb.Append(p + "MinPWM=" + H.D(c.MinPWM) + "\n");
				sb.Append(p + "MaxPWM=" + H.D(c.MaxPWM) + "\n");
				sb.Append(p + "ResolutionX=" + H.D(c.ResolutionX) + "\n");
				sb.Append(p + "ResolutionY=" + H.D(c.ResolutionY) + "\n");
				sb.Append(p + "TableWidth=" + H.D(c.TableWidth) + "\n");
				sb.Append(p + "TableHeight=" + H.D(c.TableHeight) + "\n");
				sb.Append(p + "SoftLimit=" + H.B(c.SoftLimit) + "\n");
				sb.Append(p + "AccelerationXY=" + H.D(c.AccelerationXY) + "\n");
				sb.Append(p + "WiFi_SSID=" + H.Esc(c.WiFi_SSID) + "\n");
				sb.Append(p + "WiFi_Pwd=" + H.Esc(c.WiFi_Pwd) + "\n");
				sb.Append(p + "TelnetPort=" + H.Esc(c.TelnetPort) + "\n");
				sb.Append(p + "ValidateConfig(33)=" + H.Esc((string)H.Invoke(c, "ValidateConfig", 33, (object)1)) + "\n");
			}

			// SetValueIfKeyExist on a conf pre-seeded with $110 and $30
			foreach (string l in lines)
			{
				GrblConfST c = new GrblConfST(vers["1.1"]);
				c.AddOrUpdate("$110=111"); c.AddOrUpdate("$30=222");
				bool r = (bool)H.Invoke(c, "SetValueIfKeyExist", l);
				List<string> kv = new List<string>();
				foreach (KeyValuePair<int, string> k in c) kv.Add(k.Key + "=" + H.Esc(k.Value));
				sb.Append("SetValueIfKeyExist(" + H.Esc(l) + ")=" + H.B(r) + " data=" + string.Join("|", kv.ToArray()) + "\n");
			}
			sb.Append("\n");
		}
	}
}

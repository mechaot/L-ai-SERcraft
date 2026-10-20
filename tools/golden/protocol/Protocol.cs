// Protocol oracle: drives the ORIGINAL GrblCore (Mono build) with a scripted board and records every byte
// it writes + events + state dumps. The same scenario files (.scn) are replayed on the Rust core by
// crates/core/tests/golden_protocol.rs and the traces compared byte for byte.
//
// Execution model ("stepping"): no TX/RX threads. The harness calls the private ThreadTX / ThreadRX bodies
// itself (reflection) so every interleaving is deterministic. Free-running thread scenarios use OpenCom.
// DSL reference: tools/golden/protocol/README (see top of scenarios/_dsl.txt).
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text;
using System.Threading;
using System.Windows.Forms;
using LaserGRBL;
using LaserGRBL.ComWrapper;

namespace Proto
{
	public static class Esc
	{
		// Same escape as the Rust side: \\ \n \r \t, other control chars as \xHH.
		public static string E(string s)
		{
			StringBuilder sb = new StringBuilder();
			foreach (char c in s)
			{
				if (c == '\\') sb.Append("\\\\");
				else if (c == '\n') sb.Append("\\n");
				else if (c == '\r') sb.Append("\\r");
				else if (c == '\t') sb.Append("\\t");
				else if (c < 0x20 || c == 0x7f) sb.Append("\\x" + ((int)c).ToString("X2"));
				else sb.Append(c);
			}
			return sb.ToString();
		}

		// Inverse of E (plus \xHH and \uHHHH for scenario authors).
		public static string U(string s)
		{
			StringBuilder sb = new StringBuilder();
			for (int i = 0; i < s.Length; i++)
			{
				char c = s[i];
				if (c != '\\' || i + 1 >= s.Length) { sb.Append(c); continue; }
				char n = s[++i];
				if (n == 'n') sb.Append('\n');
				else if (n == 'r') sb.Append('\r');
				else if (n == 't') sb.Append('\t');
				else if (n == '\\') sb.Append('\\');
				else if (n == 'x') { sb.Append((char)Convert.ToInt32(s.Substring(i + 1, 2), 16)); i += 2; }
				else if (n == 'u') { sb.Append((char)Convert.ToInt32(s.Substring(i + 1, 4), 16)); i += 4; }
				else { sb.Append('\\'); sb.Append(n); }
			}
			return sb.ToString();
		}
	}

	// Scripted board rule: matches written lines, answers with fixed lines.
	class Rule
	{
		public string Match; public bool Exact; public bool Once; public bool Used; public List<string> Replies = new List<string>();
	}

	class IoMarker { public Exception Ex; }

	// The fake serial link. All writes land in the rig's TX buffer; replies come from the board script.
	class ScriptCom : IComWrapper
	{
		public Rig R;
		readonly object lk = new object();
		readonly Queue<object> rxq = new Queue<object>();
		volatile bool opened;

		public void Configure(params object[] param) { }
		public void Open() { lock (lk) { opened = true; rxq.Clear(); } }
		public void Close(bool auto) { opened = false; }
		public bool IsOpen { get { return opened; } }

		public void Write(byte b) { R.TxByte(b); }
		public void Write(byte[] arr) { R.TxArr(arr); }
		public void Write(string text) { R.TxStr(text); }

		public void Push(object o) { lock (lk) rxq.Enqueue(o); }
		public int Pending { get { lock (lk) return rxq.Count; } }

		public string ReadLineBlocking()
		{
			for (;;)
			{
				lock (lk)
				{
					if (rxq.Count > 0)
					{
						object o = rxq.Dequeue();
						if (o is IoMarker) throw ((IoMarker)o).Ex;
						return (string)o;
					}
				}
				if (!IsOpen) return null;
				Thread.Sleep(1);
			}
		}

		public bool HasData() { lock (lk) return IsOpen && rxq.Count > 0; }
	}

	class Rig
	{
		public static readonly BindingFlags ALL = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static;
		public GrblCore Core; public ScriptCom Com; public Control Syncro;
		public Firmware Fw;
		public bool Threads;                         // free-running mode: replies go straight to the RX queue
		public bool ShowQueries;                     // trace '?' / 0x88 / M114
		public bool BoardAuto = true;
		public List<Rule> Rules = new List<Rule>();
		public Queue<string> BoardQ = new Queue<string>();  // stepping mode: replies waiting for a pump
		public List<string> TxBuf = new List<string>();
		public List<string> EvBuf = new List<string>();
		public int TxTotal;
		public List<string> TxAll = new List<string>();
		readonly object lk = new object();

		// ---- TX recording
		bool IsQueryByte(byte b) { return b == 0x3F || b == 0x88; }

		public void TxByte(byte b)
		{
			lock (lk) { if (ShowQueries || !IsQueryByte(b)) { TxBuf.Add("TXB " + b.ToString("X2")); TxTotal++; } }
		}

		public void TxArr(byte[] a)
		{
			StringBuilder sb = new StringBuilder("TXA");
			foreach (byte b in a) sb.Append(" " + b.ToString("X2"));
			lock (lk) { TxBuf.Add(sb.ToString()); TxTotal++; }
		}

		public void TxStr(string t)
		{
			lock (lk)
			{
				if (ShowQueries || t != "M114\n") { TxBuf.Add("TX " + Esc.E(t)); TxTotal++; }
				TxAll.Add(t);
			}
			// board sees every text write as one line
			string line = t.TrimEnd('\r', '\n');
			if (line.Length == 0) return;
			Reply(line);
		}

		void Reply(string line)
		{
			List<string> rep = null;
			lock (lk)
			{
				foreach (Rule r in Rules)
				{
					if (r.Once && r.Used) continue;
					bool hit = r.Exact ? line == r.Match : line.Contains(r.Match);
					if (!hit) continue;
					r.Used = true; rep = r.Replies; break;
				}
				if (rep == null && BoardAuto) rep = new List<string> { "ok" };
			}
			if (rep == null) return;
			foreach (string s in rep)
			{
				if (Threads) Com.Push(s);
				else lock (lk) BoardQ.Enqueue(s);
			}
		}

		public bool Seen(string sub) { lock (lk) { foreach (string x in TxAll) if (x.Contains(sub)) return true; } return false; }

		public void Ev(string s) { lock (lk) EvBuf.Add("EV " + s); }

		public void Flush(StringBuilder o)
		{
			List<string> ev, tx;
			lock (lk) { ev = new List<string>(EvBuf); tx = new List<string>(TxBuf); EvBuf.Clear(); TxBuf.Clear(); }
			foreach (string s in ev) o.Append("  " + s + "\n");
			foreach (string s in tx) o.Append("  " + s + "\n");
		}

		// ---- reflection helpers
		public static FieldInfo F(string n) { return typeof(GrblCore).GetField(n, ALL); }
		public object Get(string n) { return F(n).GetValue(Core); }
		public void Set(string n, object v) { F(n).SetValue(Core, v); }
		public object Call(string n, params object[] a)
		{
			MethodInfo found = null;
			foreach (MethodInfo m in typeof(GrblCore).GetMethods(ALL))
				if (m.Name == n && m.GetParameters().Length == a.Length) { found = m; break; }
			if (found == null) throw new MissingMethodException(n);
			try { return found.Invoke(Core, a); }
			catch (TargetInvocationException e) { throw e.InnerException; }
		}

		public static GrblCore Make(Firmware fw, Control c)
		{
			switch (fw)
			{
				case Firmware.Marlin: return new MarlinCore(c, null, null);
				case Firmware.Smoothie: return new SmoothieCore(c, null, null);
				case Firmware.VigoWork: return new VigoCore(c, null, null);
				default: return new GrblCore(c, null, null);
			}
		}
	}

	class Interp
	{
		Rig R;
		readonly StringBuilder o = new StringBuilder();
		readonly string scratch;
		Thread asyncT; Exception asyncEx;
		string prog; bool inProg; int progN;
		readonly CultureInfo inv = CultureInfo.InvariantCulture;

		public Interp(string scratch) { this.scratch = scratch; }

		public string Partial { get { return o.ToString(); } }

		public string Run(string[] lines)
		{
			ResetAll();
			foreach (string raw in lines)
			{
				string line = raw.TrimEnd('\r');
				if (inProg)
				{
					if (line.Trim() == "end")
					{
						inProg = false;
						LoadProgram(prog);
						o.Append("> prog (" + progN + " lines)\n");
					}
					else { prog += line + "\n"; progN++; }
					continue;
				}
				if (line.Trim().Length == 0 || line.TrimStart().StartsWith("#")) continue;
				if (line.Trim() == "prog") { inProg = true; prog = ""; progN = 0; continue; }
				o.Append("> " + line + "\n");
				try { Exec(line); }
				catch (Exception e)
				{
					while (e is TargetInvocationException && e.InnerException != null) e = e.InnerException;
					o.Append("  ERR " + e.GetType().Name + "\n");
				}
				if (R != null) R.Flush(o);
			}
			Shutdown();
			return o.ToString();
		}

		void ResetAll()
		{
			// settings store + static leftovers of earlier scenarios
			FieldInfo dic = typeof(Settings).GetField("dic", Rig.ALL);
			((System.Collections.IDictionary)dic.GetValue(null)).Clear();
			Type cj = typeof(GrblCore).GetNestedType("ContinuousJog");
			cj.GetField("mPrev", Rig.ALL).SetValue(null, null);
			cj.GetField("mCurr", Rig.ALL).SetValue(null, null);
			Settings.SetObject("DisableSafetyCountdown", true);
			foreach (string s in new string[] { "Success", "Warning", "Fatal", "Connect", "Disconnect" })
				Settings.SetObject("Sound." + s + ".Enabled", false);
			Settings.SetObject("Threading Mode", GrblCore.ThreadingMode.Slow);
			Settings.SetObject("GCode.CustomHeader", "G90");
			Settings.SetObject("GCode.CustomFooter", "M5");
			Settings.SetObject("GCode.CustomPasses", "G4 P0");
			CSVD.LoadAppropriateCSV(new GrblCore.GrblVersionInfo(1, 1, 'f'));
		}

		void Shutdown()
		{
			if (R != null && R.Core != null)
			{
				try { R.Core.CloseCom(true); } catch { }
			}
		}

		static string[] Split(string line, out string rest)
		{
			int sp = line.IndexOf(' ');
			string verb = sp < 0 ? line : line.Substring(0, sp);
			rest = sp < 0 ? "" : line.Substring(sp + 1);
			return new string[] { verb };
		}

		static string[] Tok(string rest)
		{
			return rest.Split(new char[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
		}

		GrblCore.GrblVersionInfo ParseVer(string t)
		{
			if (t == "none") return null;
			int dot = t.IndexOf('.');
			int maj = int.Parse(t.Substring(0, dot));
			string m = t.Substring(dot + 1);
			if (m.Length > 1) return new GrblCore.GrblVersionInfo(maj, int.Parse(m.Substring(0, 1)), m[1]);
			return new GrblCore.GrblVersionInfo(maj, int.Parse(m));
		}

		static GrblCore.ThreadingMode TM(string n)
		{
			switch (n)
			{
				case "Slow": return GrblCore.ThreadingMode.Slow;
				case "Quiet": return GrblCore.ThreadingMode.Quiet;
				case "Fast": return GrblCore.ThreadingMode.Fast;
				case "UltraFast": return GrblCore.ThreadingMode.UltraFast;
				default: return GrblCore.ThreadingMode.Insane;
			}
		}

		void LoadProgram(string text)
		{
			string path = Path.Combine(scratch, "prog_" + Interlocked.Increment(ref fileNo) + ".nc");
			File.WriteAllText(path, text, new UTF8Encoding(false));
			GrblFile f = R.Core.LoadedFile;
			bool done = false;
			GrblFile.OnFileLoadedDlg h = delegate (long el, string fn) { done = true; };
			f.OnFileLoaded += h;
			f.LoadFile(path, false);
			int n = 0;
			while (f.CheckInUse(false)) { Thread.Sleep(2); if (++n > 20000) throw new Exception("load timeout"); }
			f.OnFileLoaded -= h;
			if (!done) throw new Exception("load failed");
		}
		static int fileNo = 0;

		void StepTx()
		{
			MethodInfo m = typeof(GrblCore).GetMethod("ThreadTX", Rig.ALL);
			try { m.Invoke(R.Core, null); } catch (TargetInvocationException e) { throw e.InnerException; }
		}

		void StepRx()
		{
			MethodInfo m = typeof(GrblCore).GetMethod("ThreadRX", Rig.ALL);
			try { m.Invoke(R.Core, null); } catch (TargetInvocationException e) { throw e.InnerException; }
		}

		int TxSleep() { return ((Tools.ThreadObject)R.Get("TX")).SleepTime; }
		int RxSleep() { return ((Tools.ThreadObject)R.Get("RX")).SleepTime; }

		// one RX step with a line
		void Feed(string line)
		{
			R.Com.Push(line);
			StepRx();
		}

		// deliver one queued board reply; false when none
		bool PumpOne()
		{
			string s = null;
			lock (R.BoardQ) { if (R.BoardQ.Count == 0) return false; s = R.BoardQ.Dequeue(); }
			Feed(s);
			return true;
		}

		void Exec(string line)
		{
			string rest;
			string verb = Split(line, out rest)[0];
			string[] t = Tok(rest);
			switch (verb)
			{
				case "reset": ResetAll(); break;
				case "set": DoSet(rest); break;
				case "conf":
					{
						GrblCore.GrblVersionInfo v = ParseVer(t[0]);
						GrblConfST c = new GrblConfST(v);
						for (int i = 1; i < t.Length; i++) c.AddOrUpdate(t[i]);
						Settings.SetObject("Grbl Configuration ST", c);
						break;
					}
				case "lastver":
					{
						GrblCore.GrblVersionInfo v = ParseVer(t[0]);
						if (v == null) typeof(Settings).GetMethod("DeleteObject", Rig.ALL).Invoke(null, new object[] { "Last GrblVersion known" });
						else Settings.SetObject("Last GrblVersion known", v);
						break;
					}
				case "fw": DoFw(t[0]); break;
				case "board": DoBoard(rest); break;
				case "connect": DoConnect(false); break;
				case "fastconnect": DoConnect(true); if (t.Length > 0) { Feed(Esc.U(rest)); } break;
				case "connectthreads": R.Threads = true; R.Core.OpenCom(); break;
				case "forcestatus": R.Set("mMachineStatus", Enum.Parse(typeof(GrblCore.MacStatus), t[0])); break;
				case "rx": Feed(Esc.U(rest)); break;
				case "rxio":
					{
						int n = int.Parse(t[0]);
						string text = Esc.U(rest.Substring(rest.IndexOf(' ') + 1));
						for (int i = 0; i < n; i++) R.Com.Push(new IoMarker { Ex = new IOException("io") });
						Feed(text);
						break;
					}
				case "rxfatal":
					{
						R.Com.Push(new IoMarker { Ex = new InvalidOperationException("fatal") });
						StepRx();
						break;
					}
				case "tx":
					{
						int n = t.Length > 0 ? int.Parse(t[0]) : 1;
						for (int i = 0; i < n; i++) { StepTx(); R.Flush(o); o.Append("  TXSLEEP " + TxSleep() + "\n"); }
						break;
					}
				case "rxstep":
					StepRx(); o.Append("  RXSLEEP " + RxSleep() + "\n"); break;
				case "pump":
					{
						int n = t.Length > 0 ? int.Parse(t[0]) : int.MaxValue;
						for (int i = 0; i < n; i++) { if (!PumpOne()) break; R.Flush(o); }
						break;
					}
				case "run": DoRun(t.Length > 0 ? int.Parse(t[0]) : 5000); break;
				case "rununtil": RunWhile(delegate () { return TxSeen(Esc.U(rest)); }); break;
				case "rundone": RunWhile(RefreshIdle); break;
				case "awaitasync": asyncT.Join(); if (asyncEx != null) o.Append("  ASYNCERR " + asyncEx.GetType().Name + " " + Esc.E(asyncEx.Message) + "\n"); break;
				case "sleep": ChunkSleep(int.Parse(t[0])); break;
				case "rsleep": ChunkSleep(int.Parse(t[0])); break;
				case "rxq": R.Com.Push(Esc.U(rest)); break;
				case "trace": if (t[0] == "queries") R.ShowQueries = t[1] == "on"; break;
				case "dump": Dump(); break;
				case "dumpconf": DumpConf(); break;
				case "async": DoAsync(rest); break;
				case "join": DoJoin(); break;
				case "waitfor": DoWaitFor(t); break;
				default: DoApi(verb, rest, t); break;
			}
		}

		// HiResTimer drops gaps > 10 s between two reads (system sleep detection), so poll it while sleeping.
		static void ChunkSleep(int ms)
		{
			DateTime end = DateTime.UtcNow.AddMilliseconds(ms);
			while (true)
			{
				double left = (end - DateTime.UtcNow).TotalMilliseconds;
				if (left <= 0) break;
				Thread.Sleep((int)Math.Min(left, 200));
				long x = Tools.HiResTimer.TotalNano;
			}
		}

		void DoSet(string rest)
		{
			string[] t = Tok(rest);
			string type = t[0];
			// key is quoted when it has spaces: set bool "Reset Grbl On Connect" true
			string key; string val;
			if (rest.IndexOf('"') >= 0)
			{
				int a = rest.IndexOf('"'), b = rest.IndexOf('"', a + 1);
				key = rest.Substring(a + 1, b - a - 1);
				val = rest.Substring(b + 1).Trim();
			}
			else { key = t[1]; val = t.Length > 2 ? t[2] : ""; }
			switch (type)
			{
				case "bool": Settings.SetObject(key, val == "true"); break;
				case "int": Settings.SetObject(key, int.Parse(val)); break;
				case "str": Settings.SetObject(key, Esc.U(val)); break;
				case "sm": Settings.SetObject(key, Enum.Parse(typeof(GrblCore.StreamingMode), val)); break;
				case "tm": Settings.SetObject(key, TM(val)); break;
				case "ts": Settings.SetObject(key, TimeSpan.FromSeconds(double.Parse(val, inv))); break;
				default: throw new Exception("bad set type " + type);
			}
		}

		void DoFw(string name)
		{
			if (R != null && R.Core != null) { try { R.Core.CloseCom(true); } catch { } }
			Rig r = new Rig();
			r.Fw = name == "vigo" ? Firmware.VigoWork : name == "marlin" ? Firmware.Marlin : name == "smoothie" ? Firmware.Smoothie : Firmware.Grbl;
			r.Syncro = new Control();
			r.Core = Rig.Make(r.Fw, r.Syncro);
			r.Com = new ScriptCom { R = r };
			Rig rr = r;
			typeof(GrblCore).GetField("com", Rig.ALL).SetValue(r.Core, r.Com);
			r.Core.MachineStatusChanged += delegate () { rr.Ev("status " + rr.Core.MachineStatus); };
			r.Core.IssueDetected += delegate (GrblCore.DetectedIssue i) { rr.Ev("issue " + i); };
			r.Core.OnOverrideChange += delegate () { rr.Ev("override"); };
			r.Core.OnLoopCountChange += delegate (decimal d) { rr.Ev("loop " + d.ToString(inv)); };
			r.Core.OnProgramEnded += delegate () { rr.Ev("programended"); };
			R = r;
		}

		void DoBoard(string rest)
		{
			string[] t = Tok(rest);
			if (t[0] == "auto") { R.BoardAuto = t[1] == "on"; return; }
			if (t[0] == "clear") { R.Rules.Clear(); lock (R.BoardQ) R.BoardQ.Clear(); return; }
			if (t[0] == "rule")
			{
				// board rule [once] <match> => a | b | c      match "=text" is exact
				string body = rest.Substring(rest.IndexOf("rule") + 5);
				Rule r = new Rule();
				if (body.StartsWith("once ")) { r.Once = true; body = body.Substring(5); }
				int arrow = body.IndexOf(" => ");
				string m = Esc.U(body.Substring(0, arrow));
				string rep = body.Substring(arrow + 4);
				if (m.StartsWith("=")) { r.Exact = true; m = m.Substring(1); }
				r.Match = m;
				if (rep.Trim() != "<none>")
					foreach (string s in rep.Split('|')) r.Replies.Add(Esc.U(s.Trim()));
				R.Rules.Add(r);
				return;
			}
			throw new Exception("bad board cmd");
		}

		void DoConnect(bool fast)
		{
			GrblCore c = R.Core;
			// OpenCom without threads
			R.Set("FixCH340", false);
			R.Set("FixCH340_exception", (uint)0);
			R.Set("FixCH340_goodread", (uint)0);
			R.Set("mAutoBufferSize", 127);
			R.Call("SetStatus", GrblCore.MacStatus.Connecting);
			R.Set("connectStart", Tools.HiResTimer.TotalMilliseconds);
			if (!R.Com.IsOpen) R.Com.Open();
			R.Call("StartTX");
			if (fast) R.Set("mMachineStatus", GrblCore.MacStatus.Idle);
		}

		void DoRun(int max)
		{
			for (int i = 0; i < max; i++)
			{
				int before = R.TxTotal;
				StepTx();
				bool progress = R.TxTotal != before;
				R.Flush(o);
				if (PumpOne()) { progress = true; R.Flush(o); }
				if (!progress && R.BoardQ.Count == 0 && !CanSendNow()) break;
				if (!progress && R.BoardQ.Count == 0) break;
			}
		}

		// waits are condition based (CPU contention safe); caps are generous
		bool TxSeen(string sub) { return R.Seen(sub); }
		bool RefreshIdle() { return object.ReferenceEquals(R.Get("mQueuePtr"), R.Get("mQueue")) && object.ReferenceEquals(R.Get("mSentPtr"), R.Get("mSent")); }

		void RunWhile(Func<bool> until)
		{
			DateTime end = DateTime.UtcNow.AddSeconds(90);
			while (!until() && DateTime.UtcNow < end)
			{
				int before = R.TxTotal;
				StepTx();
				bool progress = R.TxTotal != before;
				R.Flush(o);
				if (PumpOne()) { progress = true; R.Flush(o); }
				if (!progress) Thread.Sleep(1);
			}
			if (!until()) o.Append("  RUNWHILE TIMEOUT\n");
		}

		bool CanSendNow() { return (bool)R.Call("CanSend"); }

		void DoWaitFor(string[] t)
		{
			// waitfor status <Name> | queue_empty | programdone | pending0   (free-run mode; polls up to 20 s)
			DateTime end = DateTime.UtcNow.AddSeconds(90);
			while (DateTime.UtcNow < end)
			{
				bool ok = false;
				GrblCore c = R.Core;
				if (t[0] == "status") ok = c.MachineStatus.ToString() == t[1];
				else if (t[0] == "txtext") ok = TxSeen(Esc.U(rest2(t)));
				else if (t[0] == "refreshdone") ok = RefreshIdle();
				else if (t[0] == "txlines") ok = CountTxLines() >= int.Parse(t[1]);
				else if (t[0] == "idleprog") ok = !c.InProgram && ((System.Collections.ICollection)R.Get("mQueue")).Count == 0 && ((System.Collections.ICollection)R.Get("mPending")).Count == 0 && R.Com.Pending == 0;
				else if (t[0] == "inprogram") ok = c.InProgram;
				else if (t[0] == "quiet") ok = R.Com.Pending == 0 && ((System.Collections.ICollection)R.Get("mPending")).Count == 0;
				if (ok) { Thread.Sleep(50); return; }
				Thread.Sleep(5);
			}
			o.Append("  WAITFOR TIMEOUT\n");
		}

		static string rest2(string[] t) { return string.Join(" ", t, 1, t.Length - 1); }
		int CountTxLines() { return R.TxTotal; }

		void DoAsync(string rest)
		{
			asyncEx = null;
			string[] t = Tok(rest);
			Rig r = R;
			asyncT = new Thread(delegate ()
			{
				try
				{
					switch (t[0])
					{
						case "refreshconfig": r.Core.RefreshConfig((GrblCore.RefreshCause)Enum.Parse(typeof(GrblCore.RefreshCause), t.Length > 1 ? t[1] : "OnDialog")); break;
						case "refreshinfo": r.Core.RefreshMachineInfo(); break;
						case "writeconfig":
							{
								List<GrblConfST.GrblConfParam> l = new List<GrblConfST.GrblConfParam>();
								for (int i = 1; i < t.Length; i++)
								{
									string[] kv = t[i].Split('=');
									l.Add(new GrblConfST.GrblConfParam(int.Parse(kv[0]), kv[1]));
								}
								r.Core.WriteConfig(l);
								break;
							}
						case "wifi": r.Call("WriteWiFiConfig", t[1], t[2]); break;
						default: throw new Exception("bad async op");
					}
				}
				catch (Exception e) { asyncEx = e; }
			});
			asyncT.Start();
		}

		void DoJoin()
		{
			int guard = 0;
			while (asyncT.IsAlive && guard++ < 20000)
			{
				StepTx(); R.Flush(o);
				if (!PumpOne()) Thread.Sleep(1);
				else R.Flush(o);
			}
			asyncT.Join();
			if (asyncEx != null)
			{
				o.Append("  ASYNCERR " + asyncEx.GetType().Name + " " + Esc.E(asyncEx.Message) + "\n");
			}
		}

		static string Num(float f) { return f.ToString("R", CultureInfo.InvariantCulture); }
		static string Pt(GPoint p) { return Num(p.X) + "," + Num(p.Y) + "," + Num(p.Z); }

		static string NormTime(string s)
		{
			// "[12 lines, 0 errors, 00:00:03]" -> time part replaced
			if (s.StartsWith("[") && s.EndsWith("]") && s.Contains(" lines, "))
			{
				int i = s.LastIndexOf(", ");
				return s.Substring(0, i + 2) + "<time>]";
			}
			return s;
		}

		void Dump()
		{
			GrblCore c = R.Core;
			object tp = R.Get("mTP");
			Type tpt = tp.GetType();
			TimeProjection p = (TimeProjection)tp;
			StringBuilder d = new StringBuilder("  DUMP");
			d.Append(" status=" + c.MachineStatus);
			d.Append(" inprogram=" + c.InProgram);
			d.Append(" target=" + p.Target + " sent=" + p.Sent + " executed=" + p.Executed + " errors=" + p.ErrorCount);
			d.Append(" lastissue=" + p.LastIssue);
			d.Append(" used=" + c.UsedBuffer + " bsize=" + c.BufferSize + " free=" + c.FreeBuffer);
			d.Append(" grblblock=" + c.GrblBlock + " grblbuffer=" + c.GrblBuffer);
			d.Append(" ovcur=" + c.OverrideG1 + "," + c.OverrideG0 + "," + c.OverrideS);
			d.Append(" ovtar=" + c.TOverrideG1 + "," + c.TOverrideG0 + "," + c.TOverrideS);
			d.Append(" mpos=" + Pt(c.MachinePosition) + " wco=" + Pt(c.WorkingOffset) + " lastwco=" + Pt(p.LastKnownWCO));
			d.Append(" queue=" + ((System.Collections.ICollection)R.Get("mQueue")).Count);
			d.Append(" pending=" + ((System.Collections.ICollection)R.Get("mPending")).Count);
			d.Append(" retry=" + (R.Get("mRetryQueue") != null));
			d.Append(" loop=" + c.LoopCount.ToString(inv));
			d.Append(" curf=" + Num(c.CurrentF) + " curs=" + Num(c.CurrentS));
			d.Append(" ver=" + (c.GrblVersion == null ? "none" : c.GrblVersion.ToString()));
			d.Append(" ip=" + (c.DetectedIP ?? "none"));
			d.Append(" failed=" + c.FailedConnectionCount);
			d.Append(" holduser=" + R.Get("mHoldByUserRequest") + " holdcool=" + R.Get("mHoldByCoolingRequest"));
			d.Append(" connected=" + c.IsConnected + " stuck=" + c.IsBufferStuck());
			d.Append(" canjog=" + c.JogEnabled);
			d.Append(" comopen=" + R.Com.IsOpen);
			o.Append(d + "\n");
			System.Collections.IList sent = (System.Collections.IList)R.Get("mSent");
			for (int i = 0; i < sent.Count; i++)
			{
				IGrblRow row = (IGrblRow)sent[i];
				if (row is GrblCommand)
				{
					GrblCommand g = (GrblCommand)row;
					o.Append("  SENT[" + i + "] CMD " + Esc.E(g.Command) + " | " + g.Status + " | " + Esc.E(g.GetResult(false, false) ?? "<null>") + "\n");
				}
				else
					o.Append("  SENT[" + i + "] MSG " + Esc.E(NormTime(row.GetDecodedMessage())) + "\n");
			}
		}

		void DumpConf()
		{
			GrblConfST c = GrblCore.Configuration;
			StringBuilder d = new StringBuilder("  CONF count=" + c.Count);
			List<KeyValuePair<int, string>> l = new List<KeyValuePair<int, string>>();
			foreach (KeyValuePair<int, string> kv in c) l.Add(kv);
			l.Sort(delegate (KeyValuePair<int, string> a, KeyValuePair<int, string> b) { return a.Key.CompareTo(b.Key); });
			foreach (KeyValuePair<int, string> kv in l) d.Append(" $" + kv.Key + "=" + kv.Value);
			o.Append(d + "\n");
		}

		static GrblCore.JogDirection Dir(string s) { return (GrblCore.JogDirection)Enum.Parse(typeof(GrblCore.JogDirection), s); }

		PointF PF(string x, string y) { return new PointF(float.Parse(x, inv), float.Parse(y, inv)); }
		float Speed(string s) { return float.Parse(s, inv); }

		void DoApi(string verb, string rest, string[] t)
		{
			GrblCore c = R.Core;
			switch (verb)
			{
				case "runprogram":
					{
						TimeProjection p = (TimeProjection)R.Get("mTP");
						if (c.CanSendFile && !(p.Executed == 0 || p.Executed == p.Target))
						{
							// UserWantToContinue would open ResumeJobForm: report its inputs like Rust does
							GPoint wco = (GPoint)R.Get("mWCO"); GPoint mp = c.MachinePosition;
							bool setwco = wco == GPoint.Zero && p.LastKnownWCO != GPoint.Zero;
							bool homing = mp == GPoint.Zero && p.LastIssue != GrblCore.DetectedIssue.ManualAbort && p.LastIssue != GrblCore.DetectedIssue.ManualReset;
							o.Append("  ASKRESUME executed=" + p.Executed + " sent=" + p.Sent + " target=" + p.Target + " issue=" + p.LastIssue + " homingenabled=" + GrblCore.Configuration.HomingEnabled + " sughoming=" + homing + " sugwco=" + setwco + " lastwco=" + Pt(p.LastKnownWCO) + "\n");
						}
						else if (!c.CanSendFile) o.Append("  NOTALLOWED\n");
						else c.RunProgram(null);
						break;
					}
				case "runfromstart": R.Call("RunProgramFromStart", t.Length > 0 && t[0] == "true", false, false); break;
				case "continue": R.Call("ContinueProgramFromKnown", int.Parse(t[0]), t[1] == "true", t[2] == "true"); break;
				case "abort": c.AbortProgram(); break;
				case "feedhold": c.FeedHold(t.Length > 0 && t[0] == "cooling"); break;
				case "resume": c.CycleStartResume(false); break;
				case "safetydoor": c.SafetyDoor(); break;
				case "grblreset": c.GrblReset(); break;
				case "closecom": c.CloseCom(t[0] == "user"); break;
				case "jogset": c.JogSpeed = int.Parse(t[0]); c.JogStep = decimal.Parse(t[1], inv); break;
				case "jog":
					{
						// jog <Dir> <fast|speed> [step]
						GrblCore.JogDirection d = Dir(t[0]);
						bool fast = t[1] == "fast";
						if (t.Length > 2)
						{
							decimal step = decimal.Parse(t[2], inv);
							if (fast) c.JogToDirection(d, true, step); else c.JogToDirection(d, Speed(t[1]), step);
						}
						else
						{
							if (fast) c.JogToDirection(d, true); else c.JogToDirection(d, Speed(t[1]), c.JogStep);
						}
						break;
					}
				case "jogpos":
					if (t[2] == "fast") c.JogToPosition(PF(t[0], t[1]), true); else c.JogToPosition(PF(t[0], t[1]), Speed(t[2]));
					break;
				case "cjog": c.ContinuousJogToDirection(Dir(t[0]), Speed(t[1])); break;
				case "cjogpos": c.ContinuousJogToPosition(PF(t[0], t[1]), Speed(t[2])); break;
				case "jogabort": c.JogAbort(); break;
				case "cjogabort": c.ContinuousJogAbort(); break;
				case "hkov":
					R.Call("HotKeyOverride", Enum.Parse(typeof(HotKeysManager.HotKey.Actions), "Override" + t[0]));
					break;
				case "tov":
					{
						int v = int.Parse(t[1]);
						if (t[0] == "g1") c.TOverrideG1 = v; else if (t[0] == "g0") c.TOverrideG0 = v; else c.TOverrideS = v;
						break;
					}
				case "manageov": c.ManageOverrides(); break;
				case "custom": R.Call("ExecuteCustomCode", Esc.U(rest)); break;
				case "eval": o.Append("  EVAL " + Esc.E((string)R.Call("EvaluateExpression", Esc.U(rest))) + "\n"); break;
				case "unstuck": c.UnlockFromBufferStuck(t[0] == "auto"); break;
				case "loopcount": c.LoopCount = decimal.Parse(t[0], inv); break;
				case "enqueue": c.EnqueueCommand(new GrblCommand(Esc.U(rest))); break;
				case "homing": R.Call("GrblHoming"); break;
				case "unlock": R.Call("GrblUnlock"); break;
				case "zero": R.Call("SetNewZero"); break;
				case "sendhoming": R.Call("SendHomingCommand"); break;
				case "sendunlock": R.Call("SendUnlockCommand"); break;
				case "imm": c.SendImmediate(Convert.ToByte(t[0], 16)); break;
				case "newprog": LoadProgramFromRest(rest); break;
				default: throw new Exception("unknown verb " + verb);
			}
		}

		void LoadProgramFromRest(string rest) { LoadProgram(Esc.U(rest)); }
	}

	// Differential test of the built-in Grbl 1.1 emulator: send bytes, record what comes back + its log.
	class EmuInterp
	{
		readonly StringBuilder o = new StringBuilder();
		readonly List<string> outq = new List<string>();
		long lastOut;
		int settleMs = 400;
		LaserGRBL.GrblEmulator.Grblv11Emulator emu;

		void Add(string s) { lock (outq) { outq.Add(s); lastOut = Environment.TickCount; } }

		void Settle()
		{
			// wait until nothing came for 200 ms (max 6 s)
			DateTime end = DateTime.UtcNow.AddSeconds(8);
			Thread.Sleep(60);
			while (DateTime.UtcNow < end)
			{
				long idle; lock (outq) idle = Environment.TickCount - lastOut;
				if (idle >= settleMs) break;
				Thread.Sleep(20);
			}
			List<string> l; lock (outq) { l = new List<string>(outq); outq.Clear(); }
			// threads race (queued ok vs immediate lines): compare each group sorted
			l.Sort(StringComparer.Ordinal);
			foreach (string s in l) o.Append("  " + s + "\n");
		}

		public string Run(string[] lines)
		{
			LaserGRBL.GrblEmulator.Grblv11Emulator.EmulatorMessage += delegate (string m) { Add("LOG " + (m == null ? "<null>" : Esc.E(m))); };
			emu = new LaserGRBL.GrblEmulator.Grblv11Emulator(delegate (string m) { Add("RX " + Esc.E(m)); });
			foreach (string raw in lines)
			{
				string line = raw.TrimEnd('\r');
				if (line.Trim().Length == 0 || line.StartsWith("#")) continue;
				o.Append("> " + line + "\n");
				int sp = line.IndexOf(' ');
				string verb = sp < 0 ? line : line.Substring(0, sp);
				string rest = sp < 0 ? "" : line.Substring(sp + 1);
				if (verb == "open") emu.OpenCom();
				else if (verb == "close") emu.CloseCom();
				else if (verb == "send") emu.ManageMessage(Encoding.UTF8.GetBytes(Esc.U(rest)));
				else if (verb == "byte") emu.ManageMessage(new byte[] { Convert.ToByte(rest, 16) });
				else if (verb == "wait") Thread.Sleep(int.Parse(rest));
				else if (verb == "settle") settleMs = int.Parse(rest);
				Settle();
			}
			emu.CloseCom();
			Settle();
			return o.ToString();
		}
	}

	public static class Program
	{
		public static int Main(string[] args)
		{
			Thread.CurrentThread.CurrentCulture = CultureInfo.InvariantCulture;
			Thread.CurrentThread.CurrentUICulture = CultureInfo.InvariantCulture;
			CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
			CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.InvariantCulture;

			// args: <scenarioDir> <outDir> [names...]
			string scnDir = args[0], outDir = args[1];
			string scratch = Environment.GetEnvironmentVariable("GOLDEN_TMP") ?? Path.GetTempPath();
			Directory.CreateDirectory(outDir);
			List<string> names = new List<string>();
			for (int i = 2; i < args.Length; i++) names.Add(args[i]);
			if (names.Count == 0)
				foreach (string f in Directory.GetFiles(scnDir, "*.scn")) names.Add(Path.GetFileNameWithoutExtension(f));
			names.Sort(StringComparer.Ordinal);
			int rc = 0;
			List<string> emus = new List<string>();
			foreach (string f in Directory.GetFiles(scnDir, "*.emu")) emus.Add(Path.GetFileNameWithoutExtension(f));
			emus.Sort(StringComparer.Ordinal);
			if (args.Length > 2) emus.RemoveAll(delegate (string x) { return !names.Contains(x); });
			foreach (string n in emus)
			{
				Console.WriteLine("== emu " + n);
				try
				{
					string[] lines = File.ReadAllText(Path.Combine(scnDir, n + ".emu"), new UTF8Encoding(false)).Split('\n');
					string res = new EmuInterp().Run(lines);
					File.WriteAllText(Path.Combine(outDir, n + ".emu.txt"), res, new UTF8Encoding(false));
					File.Copy(Path.Combine(scnDir, n + ".emu"), Path.Combine(outDir, n + ".emu"), true);
				}
				catch (Exception e) { Console.Error.WriteLine("EMU FAILED " + n + ": " + e); rc = 1; }
			}
			if (args.Length > 2) names.RemoveAll(delegate (string x) { return !File.Exists(Path.Combine(scnDir, x + ".scn")); });
			foreach (string n in names)
			{
				Console.WriteLine("== " + n);
				Interp ip = null;
				try
				{
					ip = new Interp(scratch);
					string[] lines = File.ReadAllText(Path.Combine(scnDir, n + ".scn"), new UTF8Encoding(false)).Split('\n');
					string res = ip.Run(lines);
					File.WriteAllText(Path.Combine(outDir, n + ".txt"), res, new UTF8Encoding(false));
					File.Copy(Path.Combine(scnDir, n + ".scn"), Path.Combine(outDir, n + ".scn"), true);
				}
				catch (Exception e) { Console.Error.WriteLine("SCENARIO FAILED " + n + ": " + e); if (ip != null) Console.Error.WriteLine(ip.Partial); rc = 1; }
			}
			Console.Out.Flush();
			Environment.Exit(rc);
			return rc;
		}
	}
}

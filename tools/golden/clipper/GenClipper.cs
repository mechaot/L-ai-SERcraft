// Golden generator for the Clipper port (WP-B). Calls the original ClipperLib inside LaserGRBL.exe
// and writes input + output of many deterministic cases to tests/golden/clipper/*.txt.
//
// Text format: one record per line, tokens separated by one space. Paths are written as
// "<n> x0 y0 x1 y1 ...". Doubles are written as 16 hex digits of their IEEE bits.
//   CASE <name>
//   CLIP <initflags>                       -> clip cases
//   ADD <polytype> <closed> <path>
//   EXEC <mode paths|tree> <cliptype> <subjfill> <clipfill>
//   RES <1|0>   | EXC
//   OUT <path>                             (paths mode, in order)
//   TOTAL <n> / CHILDCOUNT <n> / NODE <depth> <open> <hole> <path> / OPEN <path> / CLOSED <path> / ALL <path>
//   ENDEXEC
//   OFFSET <mode> <miter> <arc>            -> offset cases
//   PATH <jointype> <endtype> <path>
//   EXEC <delta>   (then RES/OUT/NODE as above)  ... ENDEXEC
//   SIMPLIFY <fill> <single>, CLEAN <dist>, MINK <kind> <closed> ... misc cases
//   END
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using ClipperLib;
using Pth = System.Collections.Generic.List<ClipperLib.IntPoint>;
using Pths = System.Collections.Generic.List<System.Collections.Generic.List<ClipperLib.IntPoint>>;

static class Gen
{
	// ---------- deterministic RNG (xorshift64*) ----------
	class Rng
	{
		ulong s;
		public Rng(ulong seed) { s = seed * 0x9E3779B97F4A7C15UL + 0x1234567UL; if (s == 0) s = 1; for (int i = 0; i < 4; i++) Next(); }
		public ulong Next()
		{
			s ^= s >> 12; s ^= s << 25; s ^= s >> 27;
			return s * 2685821657736338717UL;
		}
		public int Int(int lo, int hi) { return lo + (int)(Next() % (ulong)(hi - lo + 1)); }
		public double Dbl() { return (Next() >> 11) / 9007199254740992.0; }
		public bool Bool() { return (Next() & 1) == 1; }
	}

	static StringBuilder sb = new StringBuilder();
	static int caseCount = 0;

	static string D(double d) { return BitConverter.DoubleToInt64Bits(d).ToString("X16"); }

	static string PS(Pth p)
	{
		StringBuilder b = new StringBuilder();
		b.Append(p.Count);
		foreach (IntPoint ip in p) { b.Append(' ').Append(ip.X.ToString(CultureInfo.InvariantCulture)).Append(' ').Append(ip.Y.ToString(CultureInfo.InvariantCulture)); }
		return b.ToString();
	}

	static void Line(string s) { sb.Append(s).Append('\n'); }

	static void BeginCase(string name) { Line("CASE " + name); caseCount++; }
	static void EndCase() { Line("END"); }

	static void WriteTree(PolyTree t)
	{
		Line("TOTAL " + t.Total);
		Line("CHILDCOUNT " + t.ChildCount);
		for (PolyNode n = t.GetFirst(); n != null; n = n.GetNext())
		{
			int d = 0;
			for (PolyNode p = n.Parent; p != t; p = p.Parent) d++;
			Line("NODE " + d + " " + (n.IsOpen ? 1 : 0) + " " + (n.IsHole ? 1 : 0) + " " + PS(n.Contour));
		}
		foreach (Pth p in Clipper.OpenPathsFromPolyTree(t)) Line("OPEN " + PS(p));
		foreach (Pth p in Clipper.ClosedPathsFromPolyTree(t)) Line("CLOSED " + PS(p));
		foreach (Pth p in Clipper.PolyTreeToPaths(t)) Line("ALL " + PS(p));
	}

	static void WriteOut(Pths sol) { foreach (Pth p in sol) Line("OUT " + PS(p)); }

	// ---------- geometry generators ----------
	static IntPoint P(long x, long y) { return new IntPoint(x, y); }

	static Pth Rect(long x0, long y0, long x1, long y1, bool ccw)
	{
		Pth p = new Pth();
		p.Add(P(x0, y0)); p.Add(P(x1, y0)); p.Add(P(x1, y1)); p.Add(P(x0, y1));
		if (ccw) p.Reverse();
		return p;
	}

	static Pth Scribble(Rng r, int n, long range, long ox, long oy)
	{
		Pth p = new Pth();
		for (int i = 0; i < n; i++) p.Add(P(ox + r.Int(0, (int)range), oy + r.Int(0, (int)range)));
		return p;
	}

	static Pth Star(Rng r, int n, double rmin, double rmax, double cx, double cy)
	{
		Pth p = new Pth();
		for (int i = 0; i < n; i++)
		{
			double a = 2 * Math.PI * (i + r.Dbl() * 0.8) / n;
			double rad = rmin + (rmax - rmin) * r.Dbl();
			p.Add(new IntPoint(Math.Round(cx + rad * Math.Cos(a)), Math.Round(cy + rad * Math.Sin(a))));
		}
		return p;
	}

	static Pth Circle(double cx, double cy, double rad, int n)
	{
		Pth p = new Pth();
		for (int i = 0; i < n; i++)
		{
			double a = 2 * Math.PI * i / n;
			p.Add(new IntPoint(Math.Round(cx + rad * Math.Cos(a)), Math.Round(cy + rad * Math.Sin(a))));
		}
		return p;
	}

	static Pth GridPoly(Rng r, int n, int cells, int cell, long ox, long oy)
	{
		Pth p = new Pth();
		for (int i = 0; i < n; i++) p.Add(P(ox + r.Int(0, cells) * cell, oy + r.Int(0, cells) * cell));
		return p;
	}

	// insert midpoints (collinear) and duplicates
	static Pth Decorate(Rng r, Pth src)
	{
		Pth p = new Pth();
		for (int i = 0; i < src.Count; i++)
		{
			IntPoint a = src[i], b = src[(i + 1) % src.Count];
			p.Add(a);
			int k = r.Int(0, 3);
			if (k == 1) p.Add(a);
			else if (k == 2) p.Add(P((a.X + b.X) / 2, (a.Y + b.Y) / 2));
			else if (k == 3) { p.Add(P(a.X + (b.X - a.X) / 3, a.Y + (b.Y - a.Y) / 3)); p.Add(P(a.X + 2 * (b.X - a.X) / 3, a.Y + 2 * (b.Y - a.Y) / 3)); }
		}
		return p;
	}

	static Pth Scale(Pth src, long mul, long off)
	{
		Pth p = new Pth();
		foreach (IntPoint ip in src) p.Add(P(ip.X * mul + off, ip.Y * mul + off));
		return p;
	}

	// a "scene": returns list of subject polys and clip polys
	static void Scene(Rng r, int style, Pths subj, Pths clip)
	{
		int ns = r.Int(1, 3), nc = r.Int(1, 3);
		for (int role = 0; role < 2; role++)
		{
			Pths tgt = role == 0 ? subj : clip;
			int cnt = role == 0 ? ns : nc;
			for (int i = 0; i < cnt; i++)
			{
				long ox = r.Int(0, 60), oy = r.Int(0, 60);
				Pth p;
				switch (style)
				{
					case 0: p = Scribble(r, r.Int(3, 10), 200, ox, oy); break;
					case 1: p = Star(r, r.Int(3, 14), 20, 90, 100 + ox, 100 + oy); break;
					case 2: p = GridPoly(r, r.Int(3, 8), 10, 10, 0, 0); if (r.Bool()) p = Rect(r.Int(0, 40) * 2, r.Int(0, 40) * 2, 60 + r.Int(0, 40) * 2, 60 + r.Int(0, 40) * 2, r.Bool()); break;
					case 3: p = Decorate(r, Star(r, r.Int(3, 8), 20, 90, 100 + ox, 100 + oy)); break;
					case 4: p = Scale(r.Bool() ? Star(r, r.Int(3, 12), 20, 90, 100 + ox, 100 + oy) : Scribble(r, r.Int(3, 8), 200, ox, oy), (1L << r.Int(20, 52)) / 7 + 1, -(1L << r.Int(10, 60))); break;
					case 5: { long a = r.Int(0, 100); p = new Pth(); p.Add(P(a, 0)); p.Add(P(a + r.Int(1, 200), r.Int(1, 3))); p.Add(P(a + r.Int(0, 50), r.Int(5, 100))); } break;
					case 6: p = Star(r, r.Int(3, 10), 10, 80, r.Int(-100, 20), r.Int(-100, 20)); break;
					case 7:
						{
							// adjacent rectangles sharing edges and corners
							int gx = r.Int(0, 4), gy = r.Int(0, 4);
							p = Rect(gx * 30, gy * 30, gx * 30 + 30 * r.Int(1, 3), gy * 30 + 30 * r.Int(1, 3), r.Bool());
							break;
						}
					default: p = Circle(100 + ox, 100 + oy, r.Int(30, 90), r.Int(20, 100)); break;
				}
				tgt.Add(p);
			}
		}
		// sometimes add a hole to the first subject (opposite orientation rect inside)
		if (style == 2 && r.Bool()) { subj.Add(Rect(20, 20, 40, 40, true)); subj.Add(Rect(24, 24, 30, 30, false)); }
	}

	class Add { public PolyType t; public bool closed; public Pth p; public Add(PolyType t, bool c, Pth p) { this.t = t; closed = c; this.p = p; } }

	static void RunClip(string name, int flags, List<Add> adds, List<int[]> execs, bool[] tree)
	{
		BeginCase(name);
		Line("CLIP " + flags);
		foreach (Add a in adds) Line("ADD " + (int)a.t + " " + (a.closed ? 1 : 0) + " " + PS(a.p));
		Clipper c = new Clipper(flags);
		bool dead = false;
		try { foreach (Add a in adds) c.AddPath(a.p, a.t, a.closed); }
		catch (Exception) { dead = true; }
		for (int i = 0; i < execs.Count; i++)
		{
			int[] e = execs[i];
			Line("EXEC " + (tree[i] ? "tree" : "paths") + " " + e[0] + " " + e[1] + " " + e[2]);
			if (dead) { Line("EXC"); Line("ENDEXEC"); continue; }
			try
			{
				if (!tree[i])
				{
					Pths sol = new Pths();
					bool ok = c.Execute((ClipType)e[0], sol, (PolyFillType)e[1], (PolyFillType)e[2]);
					Line("RES " + (ok ? 1 : 0));
					WriteOut(sol);
				}
				else
				{
					PolyTree t = new PolyTree();
					bool ok = c.Execute((ClipType)e[0], t, (PolyFillType)e[1], (PolyFillType)e[2]);
					Line("RES " + (ok ? 1 : 0));
					if (ok) WriteTree(t);
				}
			}
			catch (Exception) { Line("EXC"); dead = true; }
			Line("ENDEXEC");
		}
		EndCase();
	}

	static void BoolCases()
	{
		// closed polygons, paths mode, reused clipper with several executes
		for (int i = 0; i < 150; i++)
		{
			Rng r = new Rng((ulong)(1000 + i));
			int style = i % 9;
			Pths subj = new Pths(), clip = new Pths();
			Scene(r, style, subj, clip);
			List<Add> adds = new List<Add>();
			foreach (Pth p in subj) adds.Add(new Add(PolyType.ptSubject, true, p));
			foreach (Pth p in clip) adds.Add(new Add(PolyType.ptClip, true, p));
			int[] flagsPool = { 0, 0, 0, 1, 2, 4, 3, 6, 7, 5 };
			int flags = flagsPool[r.Int(0, flagsPool.Length - 1)];
			List<int[]> ex = new List<int[]>(); List<bool> tr = new List<bool>();
			int nex = r.Int(1, 3);
			for (int k = 0; k < nex; k++)
			{
				ex.Add(new int[] { (i + k) % 4, r.Int(0, 3), r.Int(0, 3) });
				tr.Add(false);
			}
			RunClip("bool_paths_" + i, flags, adds, ex, tr.ToArray());
		}
		// same scenes, polytree output
		for (int i = 0; i < 90; i++)
		{
			Rng r = new Rng((ulong)(5000 + i));
			int style = i % 9;
			Pths subj = new Pths(), clip = new Pths();
			Scene(r, style, subj, clip);
			List<Add> adds = new List<Add>();
			foreach (Pth p in subj) adds.Add(new Add(PolyType.ptSubject, true, p));
			foreach (Pth p in clip) adds.Add(new Add(PolyType.ptClip, true, p));
			int[] flagsPool = { 0, 0, 1, 2, 4, 3, 6 };
			int flags = flagsPool[r.Int(0, flagsPool.Length - 1)];
			List<int[]> ex = new List<int[]>();
			ex.Add(new int[] { i % 4, r.Int(0, 3), r.Int(0, 3) });
			RunClip("bool_tree_" + i, flags, adds, ex, new bool[] { true });
		}
		// nested rings for tree structure (holes inside holes)
		for (int i = 0; i < 12; i++)
		{
			Rng r = new Rng((ulong)(7000 + i));
			List<Add> adds = new List<Add>();
			int rings = r.Int(2, 5);
			for (int k = 0; k < rings; k++)
				adds.Add(new Add(PolyType.ptSubject, true, Rect(k * 10, k * 10, 200 - k * 10, 200 - k * 10, (k % 2 == 1) != (i % 2 == 1))));
			if (i % 3 == 0) adds.Add(new Add(PolyType.ptSubject, true, Rect(300, 0, 400, 100, false)));
			adds.Add(new Add(PolyType.ptClip, true, Rect(r.Int(0, 50), r.Int(0, 50), 150 + r.Int(0, 100), 150 + r.Int(0, 100), false)));
			List<int[]> ex = new List<int[]>();
			ex.Add(new int[] { i % 4, i % 2 == 0 ? 0 : 1, 1 });
			ex.Add(new int[] { (i + 1) % 4, 1, 1 });
			RunClip("nested_" + i, i % 3 == 0 ? 2 : 0, adds, ex, new bool[] { true, false });
		}
		// open subject paths vs closed clips (tree)
		for (int i = 0; i < 70; i++)
		{
			Rng r = new Rng((ulong)(9000 + i));
			Pths subj = new Pths(), clip = new Pths();
			Scene(r, i % 9 == 4 ? 1 : i % 9, new Pths(), clip);
			List<Add> adds = new List<Add>();
			int nopen = r.Int(1, 5);
			for (int k = 0; k < nopen; k++)
			{
				int mode = r.Int(0, 3);
				Pth p;
				if (mode == 0) p = Scribble(r, r.Int(2, 6), 250, 0, 0);
				else if (mode == 1) { long y = r.Int(0, 200); p = new Pth(); p.Add(P(r.Int(-20, 50), y)); p.Add(P(r.Int(100, 260), y)); }
				else if (mode == 2) { long x = r.Int(0, 200); p = new Pth(); p.Add(P(x, r.Int(-20, 50))); p.Add(P(x, r.Int(100, 260))); }
				else { p = new Pth(); long x = r.Int(0, 50); long y = r.Int(0, 200); for (int z = 0; z < r.Int(3, 8); z++) { p.Add(P(x, y)); if (z % 2 == 0) x += r.Int(10, 60); else y += r.Int(-40, 40); } }
				adds.Add(new Add(PolyType.ptSubject, false, p));
			}
			if (i % 5 == 0) foreach (Pth p in subj) adds.Add(new Add(PolyType.ptSubject, true, p));
			foreach (Pth p in clip) adds.Add(new Add(PolyType.ptClip, true, p));
			List<int[]> ex = new List<int[]>();
			ex.Add(new int[] { i % 4, i % 3 == 0 ? 0 : r.Int(0, 3), r.Int(0, 3) });
			List<bool> tr = new List<bool>(); tr.Add(true);
			if (i % 7 == 0) { ex.Add(new int[] { 0, 0, 0 }); tr.Add(false); } // must raise: open paths need polytree
			RunClip("open_" + i, i % 6 == 0 ? 4 : 0, adds, ex, tr.ToArray());
		}
		// open path edge cases
		{
			List<Add> adds = new List<Add>();
			adds.Add(new Add(PolyType.ptSubject, false, new Pth { P(0, 0), P(0, 0) })); // duplicate -> skipped
			adds.Add(new Add(PolyType.ptSubject, false, new Pth { P(0, 50), P(100, 50), P(100, 50), P(200, 50) })); // horizontal chain
			adds.Add(new Add(PolyType.ptSubject, false, new Pth { P(10, 10), P(10, 90), P(10, 10) })); // back and forth
			adds.Add(new Add(PolyType.ptSubject, false, new Pth { P(0, 0), P(100, 0), P(100, 100), P(0, 100), P(0, 0) })); // closed loop as open
			adds.Add(new Add(PolyType.ptClip, true, Rect(20, 20, 80, 80, false)));
			for (int ct = 0; ct < 4; ct++)
			{
				List<int[]> ex = new List<int[]>(); ex.Add(new int[] { ct, 0, 0 });
				RunClip("open_edge_" + ct, 0, adds, ex, new bool[] { true });
			}
			// open path added as clip: must throw
			List<Add> bad = new List<Add>(); bad.Add(new Add(PolyType.ptClip, false, new Pth { P(0, 0), P(10, 10) }));
			RunClip("open_clip_throws", 0, bad, new List<int[]> { new int[] { 0, 0, 0 } }, new bool[] { true });
			// out of range coordinate
			List<Add> big = new List<Add>(); big.Add(new Add(PolyType.ptSubject, true, new Pth { P(0, 0), P(long.MaxValue / 2 + 1, 5), P(10, 100) }));
			RunClip("range_throws", 0, big, new List<int[]> { new int[] { 0, 0, 0 } }, new bool[] { false });
			List<Add> edge = new List<Add>(); edge.Add(new Add(PolyType.ptSubject, true, new Pth { P(-(1L << 62), -(1L << 62)), P(1L << 62, -(1L << 62)), P(0, 1L << 62) }));
			edge.Add(new Add(PolyType.ptClip, true, new Pth { P(-(1L << 61), -(1L << 61)), P(1L << 61, -(1L << 61)), P(1L << 61, 1L << 61) }));
			for (int ct = 0; ct < 4; ct++) RunClip("range_edge_" + ct, 0, edge, new List<int[]> { new int[] { ct, 0, 0 } }, new bool[] { false });
			// empty and degenerate input
			RunClip("empty", 0, new List<Add>(), new List<int[]> { new int[] { 1, 0, 0 } }, new bool[] { false });
			List<Add> deg = new List<Add>(); deg.Add(new Add(PolyType.ptSubject, true, new Pth { P(0, 0), P(10, 0), P(20, 0) }));
			deg.Add(new Add(PolyType.ptSubject, true, new Pth { P(0, 0), P(5, 5) }));
			deg.Add(new Add(PolyType.ptSubject, true, new Pth { P(1, 1), P(1, 1), P(1, 1) }));
			RunClip("degenerate", 0, deg, new List<int[]> { new int[] { 1, 0, 0 } }, new bool[] { false });
		}
	}

	// ---------- PotraceClipper patterns ----------
	const double resolution = 1000.0;

	static void Seg(Pths target, double x1, double y1, double x2, double y2, bool invert)
	{
		if (!invert) target.Add(new Pth { new IntPoint(x1 * resolution, y1 * resolution), new IntPoint(x2 * resolution, y2 * resolution) });
		else target.Add(new Pth { new IntPoint(x2 * resolution, y2 * resolution), new IntPoint(x1 * resolution, y1 * resolution) });
	}
	static void Pp(Pth t, double x, double y) { t.Add(new IntPoint(x * resolution, y * resolution)); }

	static void Hilbert(List<float[]> list, int depth, float dx, float dy)
	{
		float[] l;
		if (depth > 1) Hilbert(list, depth - 1, dy, dx);
		l = list[list.Count - 1]; list.Add(new float[] { l[0] + dx, l[1] + dy });
		if (depth > 1) Hilbert(list, depth - 1, dx, dy);
		l = list[list.Count - 1]; list.Add(new float[] { l[0] + dy, l[1] + dx });
		if (depth > 1) Hilbert(list, depth - 1, dx, dy);
		l = list[list.Count - 1]; list.Add(new float[] { l[0] - dx, l[1] - dy });
		if (depth > 1) Hilbert(list, depth - 1, -dy, -dx);
	}

	// same loops as PotraceClipper.AddGridSubject (dir names instead of enum); hilbert depth is a parameter
	static Pths GridSubject(string dir, double w, double h, double step, int hn)
	{
		double dstep = step * Math.Sqrt(2);
		double rdstep = step * (1 / Math.Sqrt(2));
		Pths paths = new Pths();
		if (dir == "Horizontal" || dir == "Grid") for (int y = 1; y < (h / step); y++) Seg(paths, 0, y * step, w, y * step, y % 2 == 1);
		if (dir == "Vertical" || dir == "Grid") for (int x = 1; x < (w / step); x++) Seg(paths, x * step, 0, x * step, h, x % 2 == 0);
		if (dir == "Diagonal" || dir == "DiagonalGrid") for (int i = 0; i < (w + h) / dstep; i++) Seg(paths, 0, i * dstep, i * dstep, 0, i % 2 == 0);
		if (dir == "ReverseDiagonal" || dir == "DiagonalGrid") for (int i = 0; i < (w + h) / dstep; i++) Seg(paths, 0, h - (i * dstep), i * dstep, h, i % 2 == 1);
		if (dir == "Cross")
		{
			double cl = step / 3;
			for (int y = 1; y < (h / step); y++) for (int x = 1; x < (w / step); x++)
				{
					Seg(paths, (x * step) - cl, y * step, (x * step) + cl, y * step, y % 2 == 1);
					Seg(paths, x * step, (y * step) - cl, x * step, (y * step) + cl, x % 2 == 0);
				}
		}
		if (dir == "DiagonalCross")
		{
			double cl = rdstep / 3;
			for (int y = 0; y < (h / step) + 1; y++) for (int x = 0; x < (w / step) + 1; x++)
				{
					Seg(paths, (x * step) - cl, (y * step) - cl, (x * step) + cl, (y * step) + cl, x % 2 == 1);
					Seg(paths, (x * step) + cl, (y * step) - cl, (x * step) - cl, (y * step) + cl, x % 2 == 1);
				}
		}
		if (dir == "Squares")
		{
			double cl = step / 3;
			for (int y = 0; y < (h / step) + 1; y++) for (int x = 0; x < (w / step) + 1; x++)
				{
					Pth list = new Pth();
					Pp(list, (x * step) - cl, y * step); Pp(list, x * step, (y * step) + cl); Pp(list, (x * step) + cl, y * step);
					Pp(list, x * step, (y * step) - cl); Pp(list, (x * step) - cl, y * step);
					paths.Add(list);
				}
		}
		if (dir == "ZigZag")
		{
			double hs = dstep / 2;
			for (int i = 1; i < (w + h) / dstep; i++)
			{
				Pth list = new Pth();
				if (i % 2 == 1)
				{
					double my = 0; double mx = (i * dstep) - hs;
					Pp(list, mx, my);
					while (mx > 0) { Pp(list, mx, my += hs); Pp(list, mx -= hs, my); }
				}
				else
				{
					double mx = 0; double my = (i * dstep) - hs;
					Pp(list, mx - 1, my + 1); Pp(list, mx, my);
					while (my > 0) { Pp(list, mx += hs, my); Pp(list, mx, my -= hs); }
				}
				paths.Add(list);
			}
		}
		if (dir == "Hilbert")
		{
			float ts = (float)(Math.Pow(2, hn) * step);
			List<float[]> texel = new List<float[]>();
			texel.Add(new float[] { 0, 0 });
			Hilbert(texel, hn, (float)step, 0);
			for (int y = 0; y < (h / ts); y++) for (int x = 0; x < (w / ts); x++)
				{
					Pth list = new Pth();
					for (int i = 0; i < texel.Count; i++) Pp(list, texel[i][0] + (x * ts), texel[i][1] + (y * ts));
					paths.Add(list);
				}
		}
		return paths;
	}

	// flattened polygon in mm like the GDI+ flatten: float coords * 1000, truncating ctor
	static Pth MmPoly(Pth mm1000) { return mm1000; }

	static List<Pth> PotraceShapes(int k)
	{
		List<Pth> l = new List<Pth>();
		switch (k)
		{
			case 0: l.Add(Circle(30000, 25000, 20000, 48)); break;
			case 1: l.Add(Rect(5000, 5000, 55000, 45000, false)); break;
			case 2: l.Add(Rect(5000, 5000, 55000, 45000, false)); l.Add(Rect(20000, 15000, 40000, 35000, true)); break;
			case 3: l.Add(Star(new Rng(77), 40, 8000, 24000, 30000, 25000)); break;
			case 4: l.Add(Circle(20000, 20000, 15000, 60)); l.Add(Circle(40000, 25000, 12000, 50)); break;
			case 5: { Pth p = new Pth(); for (int i = 0; i < 9; i++) p.Add(new IntPoint(10000 + (float)(i * 4.37) * 1000, 10000 + (float)((i * 7) % 5 * 3.1) * 1000)); l.Add(p); } break;
			default: l.Add(Circle(30000, 25000, 22000, 90)); l.Add(Circle(30000, 25000, 9000, 40)); break;
		}
		return l;
	}

	static void PotraceCases()
	{
		string[] dirs = { "Horizontal", "Vertical", "Diagonal", "ReverseDiagonal", "Grid", "DiagonalGrid", "Cross", "DiagonalCross", "Squares", "ZigZag", "Hilbert" };
		double[] steps = { 1.0, 2.5, 4.0 };
		int n = 0;
		foreach (string dir in dirs)
		{
			for (int si = 0; si < steps.Length; si++)
			{
				if (dir == "Hilbert" && si == 2) continue;
				double step = steps[si];
				int shape = (n + si) % 7;
				int hn = si == 0 ? 3 : 2;
				double w = 60, h = 50;
				if (dir == "Hilbert") { hn = si == 0 ? 4 : 3; }
				Pths subj = GridSubject(dir, w, h, step, hn);
				List<Add> adds = new List<Add>();
				foreach (Pth p in subj) adds.Add(new Add(PolyType.ptSubject, false, p));
				foreach (Pth p in PotraceShapes(shape)) adds.Add(new Add(PolyType.ptClip, true, p));
				// PotraceClipper: new Clipper(); Execute(ctIntersection, PolyTree, pftEvenOdd, pftEvenOdd)
				RunClip("potrace_grid_" + dir + "_" + si, 0, adds, new List<int[]> { new int[] { 0, 0, 0 } }, new bool[] { true });
				n++;
			}
		}
		// one full-size Hilbert texel with n = 6 as in the app
		{
			Pths subj = GridSubject("Hilbert", 30, 30, 0.45, 6);
			List<Add> adds = new List<Add>();
			foreach (Pth p in subj) adds.Add(new Add(PolyType.ptSubject, false, p));
			foreach (Pth p in PotraceShapes(0)) adds.Add(new Add(PolyType.ptClip, true, p));
			RunClip("potrace_hilbert6", 0, adds, new List<int[]> { new int[] { 0, 0, 0 } }, new bool[] { true });
		}
	}

	// ---------- offset ----------
	class OPath { public int jt, et; public Pth p; public OPath(int j, int e, Pth p) { jt = j; et = e; this.p = p; } }

	static void RunOffset(string name, bool tree, double miter, double arc, List<OPath> paths, double[] deltas)
	{
		BeginCase(name);
		Line("OFFSET " + (tree ? "tree" : "paths") + " " + D(miter) + " " + D(arc));
		foreach (OPath o in paths) Line("PATH " + o.jt + " " + o.et + " " + PS(o.p));
		ClipperOffset c = new ClipperOffset(miter, arc);
		foreach (OPath o in paths) c.AddPath(o.p, (JoinType)o.jt, (EndType)o.et);
		foreach (double d in deltas)
		{
			Line("EXEC " + D(d));
			try
			{
				if (!tree) { Pths sol = new Pths(); c.Execute(ref sol, d); WriteOut(sol); Line("RES 1"); }
				else { PolyTree t = new PolyTree(); c.Execute(ref t, d); Line("RES 1"); WriteTree(t); }
			}
			catch (Exception) { Line("EXC"); }
			Line("ENDEXEC");
		}
		EndCase();
	}

	static void OffsetCases()
	{
		double[] miters = { 2.0, 2.0, 1.0, 3.0, 10.0, 0.5 };
		double[] arcs = { 0.25, 0.25, 0.1, 1.0, 0.0, 5.0 };
		// closed polygons, all join types, +/- deltas
		for (int i = 0; i < 90; i++)
		{
			Rng r = new Rng((ulong)(20000 + i));
			Pths subj = new Pths(), clip = new Pths();
			int style = i % 9;
			if (style == 4) style = 1;
			Scene(r, style, subj, clip);
			List<OPath> ps = new List<OPath>();
			int jt = i % 3;
			foreach (Pth p in subj) ps.Add(new OPath(r.Int(0, 3) == 0 ? r.Int(0, 2) : jt, 0, p));
			if (i % 4 == 1) ps.Add(new OPath(jt, 1, clip[0])); // closed line
			double sc = (style == 1 || style == 8) ? 1.0 : 1.0;
			double[] deltas = i % 2 == 0
				? new double[] { 1 * sc, 3.5 * sc, 10 * sc, -2 * sc, -8.25 * sc }
				: new double[] { -1 * sc, -5 * sc, -15 * sc, -40 * sc, 6 * sc, 0.0 };
			RunOffset("offset_closed_" + i, i % 5 == 3, miters[i % miters.Length], arcs[(i / 2) % arcs.Length], ps, deltas);
		}
		// open paths with all end types
		for (int i = 0; i < 60; i++)
		{
			Rng r = new Rng((ulong)(30000 + i));
			List<OPath> ps = new List<OPath>();
			int cnt = r.Int(1, 3);
			for (int k = 0; k < cnt; k++)
			{
				Pth p = new Pth();
				int np = r.Int(1, 7);
				if (r.Int(0, 6) == 0) np = 1;
				long x = r.Int(0, 100), y = r.Int(0, 100);
				for (int z = 0; z < np; z++) { p.Add(P(x, y)); x += r.Int(-40, 60); y += r.Int(-40, 60); }
				ps.Add(new OPath(r.Int(0, 2), 2 + r.Int(0, 2), p));
			}
			double[] deltas = { 2, 7.5, 20, -3, 0.2 };
			RunOffset("offset_open_" + i, i % 6 == 2, miters[i % miters.Length], arcs[i % arcs.Length], ps, deltas);
		}
		// single points
		for (int i = 0; i < 6; i++)
		{
			List<OPath> ps = new List<OPath>();
			ps.Add(new OPath(i % 3, 2 + i % 3, new Pth { P(50, 50) }));
			ps.Add(new OPath(i % 3, 0, new Pth { P(0, 0), P(0, 0), P(0, 0) }));
			ps.Add(new OPath(i % 3, 0, new Pth { P(10, 0), P(20, 5) }));
			RunOffset("offset_points_" + i, false, 2.0, 0.25, ps, new double[] { 10, -10, 1e-30 });
		}
		// PotraceClipper inset filling: jtRound + etClosedPolygon, delta = -spacing * i * 1000 until empty
		for (int shape = 0; shape < 7; shape++)
		{
			foreach (double spacing in new double[] { 0.5, 1.0, 2.0 })
			{
				List<OPath> ps = new List<OPath>();
				foreach (Pth p in PotraceShapes(shape)) ps.Add(new OPath(1, 0, p));
				List<double> ds = new List<double>();
				// replicate loop termination using the original library
				ClipperOffset c = new ClipperOffset();
				foreach (OPath o in ps) c.AddPath(o.p, JoinType.jtRound, EndType.etClosedPolygon);
				for (int i = 1; i < 200; i++)
				{
					double d = -spacing * i * resolution;
					Pths sol = new Pths();
					c.Execute(ref sol, d);
					ds.Add(d);
					if (sol.Count == 0) break;
				}
				RunOffset("potrace_inset_" + shape + "_" + spacing.ToString(CultureInfo.InvariantCulture), false, 2.0, 0.25, ps, ds.ToArray());
			}
		}
		// large scale offsets (full range) and tiny arc tolerance
		for (int i = 0; i < 6; i++)
		{
			Rng r = new Rng((ulong)(40000 + i));
			List<OPath> ps = new List<OPath>();
			ps.Add(new OPath(i % 3, 0, Scale(Star(r, 12, 20, 90, 100, 100), 1L << 33, 0)));
			RunOffset("offset_big_" + i, false, 2.0, 1e7, ps, new double[] { 1e9, -2e9, 5e10 });
		}
	}

	// ---------- misc ----------
	static void MiscCases()
	{
		// simplify
		for (int i = 0; i < 30; i++)
		{
			Rng r = new Rng((ulong)(50000 + i));
			BeginCase("simplify_" + i);
			bool single = i % 2 == 0;
			int fill = r.Int(0, 3);
			Line("SIMPLIFY " + fill + " " + (single ? 1 : 0));
			Pths ps = new Pths();
			int cnt = single ? 1 : r.Int(2, 4);
			for (int k = 0; k < cnt; k++)
				ps.Add(i % 3 == 0 ? Scribble(r, r.Int(4, 12), 100, 0, 0) : Scale(Star(r, r.Int(5, 12), 10, 50, 60, 60), 1, 0));
			foreach (Pth p in ps) Line("PATH 0 0 " + PS(p));
			try
			{
				Pths res = single ? Clipper.SimplifyPolygon(ps[0], (PolyFillType)fill) : Clipper.SimplifyPolygons(ps, (PolyFillType)fill);
				Line("RES 1"); WriteOut(res);
			}
			catch (Exception) { Line("EXC"); }
			EndCase();
		}
		// clean polygon(s)
		for (int i = 0; i < 20; i++)
		{
			Rng r = new Rng((ulong)(60000 + i));
			BeginCase("clean_" + i);
			double dist = (new double[] { 1.415, 1.415, 0.5, 3.0, 10.0, 0.0 })[i % 6];
			Line("CLEAN " + D(dist));
			Pths ps = new Pths();
			int cnt = r.Int(1, 3);
			for (int k = 0; k < cnt; k++)
			{
				Pth p = i % 2 == 0 ? Decorate(r, Circle(100, 100, 50, r.Int(10, 80))) : Scribble(r, r.Int(0, 12), 30, 0, 0);
				// add noise points close to each other
				if (r.Bool()) { Pth q = new Pth(); foreach (IntPoint ip in p) { q.Add(ip); q.Add(P(ip.X + r.Int(0, 1), ip.Y + r.Int(0, 1))); } p = q; }
				ps.Add(p);
			}
			foreach (Pth p in ps) Line("PATH 0 0 " + PS(p));
			Pths res = Clipper.CleanPolygons(ps, dist);
			Line("RES 1"); WriteOut(res);
			// single-path overload
			Line("EXEC1");
			Line("OUT " + PS(Clipper.CleanPolygon(ps[0], dist)));
			EndCase();
		}
		// minkowski
		for (int i = 0; i < 24; i++)
		{
			Rng r = new Rng((ulong)(70000 + i));
			BeginCase("minkowski_" + i);
			int kind = i % 4; // 0 sum open/closed path, 1 sum closed, 2 sum paths, 3 diff
			bool closed = i % 2 == 0;
			Pth pattern = i % 3 == 0 ? Rect(-5, -5, 5, 5, false) : Circle(0, 0, 6, r.Int(4, 10));
			Pth path = i % 2 == 0 ? Star(r, r.Int(3, 8), 20, 50, 60, 60) : Scribble(r, r.Int(2, 6), 100, 0, 0);
			Pths paths = new Pths { path, Scale(Star(r, 5, 10, 20, 30, 30), 1, 100) };
			Line("MINK " + kind + " " + (closed ? 1 : 0));
			Line("PATTERN " + PS(pattern));
			foreach (Pth p in (kind == 2 ? paths : new Pths { path })) Line("PATH 0 0 " + PS(p));
			try
			{
				Pths res;
				if (kind == 3) res = Clipper.MinkowskiDiff(pattern, path);
				else if (kind == 2) res = Clipper.MinkowskiSum(pattern, paths, closed);
				else res = Clipper.MinkowskiSum(pattern, path, closed);
				Line("RES 1"); WriteOut(res);
			}
			catch (Exception) { Line("EXC"); }
			EndCase();
		}
		// area / orientation / pip / reverse / double-ctor
		{
			BeginCase("area_orientation_pip");
			Rng r = new Rng(80000);
			for (int i = 0; i < 40; i++)
			{
				Pth p = i % 2 == 0 ? Scribble(r, r.Int(0, 9), 100, -30, -30) : Star(r, r.Int(3, 12), 10, 60, 0, 0);
				Line("AREA " + PS(p) + " " + D(Clipper.Area(p)) + " " + (Clipper.Orientation(p) ? 1 : 0));
				for (int k = 0; k < 6; k++)
				{
					IntPoint q = P(r.Int(-60, 100), r.Int(-60, 100));
					if (k == 0 && p.Count > 0) q = p[r.Int(0, p.Count - 1)]; // vertex -> boundary
					if (k == 1 && p.Count > 1) q = P((p[0].X + p[1].X) / 2, (p[0].Y + p[1].Y) / 2);
					Line("PIP " + q.X + " " + q.Y + " " + PS(p) + " " + Clipper.PointInPolygon(q, p));
				}
			}
			EndCase();
			BeginCase("intpoint_double_ctor");
			double[] vals = { 0, 0.4, 0.5, 0.9999, -0.4, -0.5, -0.9999, 1.5, -1.5, 123456.789, -123456.789, 1e18, -1e18, 9.3e18, -9.3e18, double.NaN, double.PositiveInfinity, double.NegativeInfinity, 4294967296.5 };
			foreach (double a in vals) foreach (double b in new double[] { 0.7, -2.7 })
				{
					IntPoint ip = new IntPoint(a, b);
					Line("DCTOR " + D(a) + " " + D(b) + " " + ip.X + " " + ip.Y);
				}
			EndCase();
			BeginCase("reverse_paths_bounds");
			Pths ps = new Pths { Scribble(r, 5, 50, 0, 0), new Pth(), Scribble(r, 3, 50, -20, -20) };
			foreach (Pth p in ps) Line("PATH 0 0 " + PS(p));
			Pths copy = new Pths(); foreach (Pth p in ps) copy.Add(new Pth(p));
			Clipper.ReversePaths(copy);
			foreach (Pth p in copy) Line("OUT " + PS(p));
			IntRect rb = Clipper.GetBounds(ps);
			Line("BOUNDS " + rb.left + " " + rb.top + " " + rb.right + " " + rb.bottom);
			IntRect b0 = Clipper.GetBounds(new Pths());
			Line("BOUNDS " + b0.left + " " + b0.top + " " + b0.right + " " + b0.bottom);
			EndCase();
		}
	}

	static void Flush(string dir, string file)
	{
		File.WriteAllText(Path_(dir, file), sb.ToString());
		Console.WriteLine(file + ": " + caseCount + " cases");
		sb = new StringBuilder();
		caseCount = 0;
	}

	static string Path_(string dir, string file) { return System.IO.Path.Combine(dir, file); }

	static int Main(string[] args)
	{
		string dir = args.Length > 0 ? args[0] : ".";
		Directory.CreateDirectory(dir);
		BoolCases(); Flush(dir, "clip.txt");
		PotraceCases(); Flush(dir, "potrace_clip.txt");
		OffsetCases(); Flush(dir, "offset.txt");
		MiscCases(); Flush(dir, "misc.txt");
		return 0;
	}
}

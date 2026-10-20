// Golden generator for lg-dotnet. Run under Mono (mcs). Output dir = args[0].
// Every line is TSV. Doubles/floats are stored as raw bits (hex) for exact round trip.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

static class Gen
{
    static CultureInfo Inv = CultureInfo.InvariantCulture;
    const string NONE = "<none>";

    // decimal -> text incl. sign of negative zero (ToString drops it)
    static string DS(decimal x)
    {
        string t = x.ToString(Inv);
        return (decimal.GetBits(x)[3] < 0 && !t.StartsWith("-")) ? "-" + t : t;
    }

    static string Hex(double d) { return BitConverter.DoubleToInt64Bits(d).ToString("x16"); }
    static string HexF(float f) { return BitConverter.ToInt32(BitConverter.GetBytes(f), 0).ToString("x8"); }

    // escape strings for TSV: \\ \t \n \r and non-ASCII-printable as \uXXXX
    static string Esc(string s)
    {
        StringBuilder sb = new StringBuilder();
        foreach (char c in s)
        {
            if (c == '\\') sb.Append("\\\\");
            else if (c < 0x20 || c > 0x7e) sb.Append("\\u" + ((int)c).ToString("x4"));
            else sb.Append(c);
        }
        return sb.ToString();
    }

    static readonly string[] DoubleFmts = {
        NONE, "0", "0.0", "0.00", "0.000", "0.0000", "0.###", "0.##", "#0.00", "0.0#", "#,##0", "00", "000",
        "000000000", "00000", "00000.##", "000.##", "00.000", "+00.0;-00.0", "+00.00000;-00.00000",
        "F", "F0", "F1", "F2", "F3", "F6", "F10", "N", "N0", "N2", "P", "P1", "P0", "G", "G5", "G10", "G17", "E", "E3", "e2", "R",
        "0.00%", "#,##0.00", "0.0,,", "0.0,", "'x'0.0", "\"a\"0.0", "#", "#.##", ".0", ".##", "0.0E+00", "0.##E-0", "0.00E000", "0.0e+0",
        "0.0;(0.0)", "0.0;-0.0;zero", "0;-0;;", "0.0;;", "#,#", "#,0.0", "0,0", "0,,0", "0\\.0", "0.0‰", "00.00.00", "0,0.0,",
        "###-##", "0 0 0", "#,##0.0##", "0.00 'u'", "%0", "0%", "#.#E+0"
    };

    static double[] SpecialDoubles()
    {
        List<double> l = new List<double>();
        double[] basic = {
            0.0, -0.0, 1, -1, 0.5, -0.5, 1.5, 2.5, -2.5, 0.1, 0.2, 0.1 + 0.2, 1.0 / 3, 2.0 / 3, 10.0 / 3, 100.0 / 7,
            0.0005, 0.0015, 0.0025, 0.00049999, 0.0004, -0.0004, -0.0001, -0.0005, -0.005, 1.0005, 1.0015, 1.0045, 1.005, 2.675, 1.115, 8.345,
            0.045, 0.055, 0.995, 9.995, 99.995, 999.9995, 9.9999, 99.9999, 0.9995, 0.99949, 0.09995, 123.456, 123.4567, 1234.5678, 12345.6789,
            123456789.123456789, 1e-5, 1e-4, 9.99e-5, 1e-6, 1.5e-7, 1e-10, 1e-15, 1e-20, 1e-100, 1e-300, 1e-320, 5e-324, 1e5, 1e10, 1e14, 99999999999999.9, 123456789012345.0, 999999999999999.0,
            1e15, 1.5e15, 1e16, 1e17, 1e20, 1e21, 1e22, 1e100, 1e300, double.MaxValue, double.MinValue, double.Epsilon,
            double.NaN, double.PositiveInfinity, double.NegativeInfinity, 123456789012345678.0, 0.000123456789012345678,
            12345.678901234567, 1234567.1234567, 0.30000000000000004, 100, 1000, 10000, 100000, 1000000, 999999.5, 1234567.5, 12.5, 13.5, 0.125, 0.375,
            100.0, 25.4, 3.14159265358979, 0.1 * 3, 4.35 * 100, 1.1 * 1.1, 59.99999999, 0.99999999999999, 0.999999999999999, 0.9999999999999999,
            -1234.5678, -0.05, -0.0049, -0.00049, 255, 254.99999, 0.7, 0.07, 0.007, 15.00000000000001, 1.000000000000005, 2.000000000000005,
            123456789012345.5, 12345678901234.55, 7.0E-05, 0.0001, 0.00001234,
            1500, 2500, 0.15, 0.25, 0.35, 0.45, 1.45, 2.55, 3.65, 5e-5, 4.9999e-5, 0.1 + 0.7, 6.02214076e23, 1.602176634e-19
        };
        l.AddRange(basic);
        return l.ToArray();
    }

    static List<double> AllDoubles()
    {
        List<double> l = new List<double>(SpecialDoubles());
        Random r = new Random(20240607);
        // g-code like values: few decimals
        for (int i = 0; i < 250; i++)
        {
            int dec = r.Next(0, 8);
            double v = Math.Round(r.NextDouble() * Math.Pow(10, r.Next(0, 5)), dec);
            if (r.Next(2) == 0) v = -v;
            l.Add(v);
        }
        // midpoints x.xxx5 via decimal
        for (int i = 0; i < 150; i++)
        {
            decimal m = (decimal)r.Next(0, 100000) / 10000m + 0.00005m;
            if (r.Next(2) == 0) m = -m;
            l.Add((double)m);
            l.Add((double)(m * 10m));
            l.Add((double)(m / 10m));
        }
        // random magnitude
        for (int i = 0; i < 250; i++)
        {
            double v = (r.NextDouble() + 0.01) * Math.Pow(10, r.Next(-12, 18));
            if (r.Next(2) == 0) v = -v;
            l.Add(v);
        }
        // raw bit patterns (finite only)
        for (int i = 0; i < 60; i++)
        {
            long bits = ((long)r.Next() << 32) ^ (long)r.Next() ^ ((long)r.Next() << 16);
            double v = BitConverter.Int64BitsToDouble(bits);
            if (double.IsNaN(v) || double.IsInfinity(v)) continue;
            l.Add(v);
        }
        // arithmetic results
        for (int i = 0; i < 100; i++)
        {
            double a = Math.Round(r.NextDouble() * 100, r.Next(0, 5)), b = Math.Round(r.NextDouble() * 10, r.Next(1, 4));
            l.Add(a * b); l.Add(a / (b + 1)); l.Add(a + b); l.Add(a - b);
        }
        return l;
    }

    static string TryFmt(Func<string> f)
    {
        try { return f(); } catch (Exception) { return "EXC"; }
    }

    static void DoubleTable(string dir)
    {
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "double.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            foreach (double d in AllDoubles())
                foreach (string f in DoubleFmts)
                {
                    double dd = d;
                    string fmt = f;
                    string res = TryFmt(() => fmt == NONE ? dd.ToString(Inv) : dd.ToString(fmt, Inv));
                    w.WriteLine(Hex(d) + "\t" + Esc(f) + "\t" + Esc(res));
                }
        }
    }

    static void FloatTable(string dir)
    {
        List<float> fl = new List<float>();
        foreach (double d in SpecialDoubles()) fl.Add((float)d);
        Random r = new Random(777);
        for (int i = 0; i < 200; i++)
        {
            float v = (float)(Math.Round(r.NextDouble() * Math.Pow(10, r.Next(0, 5)), r.Next(0, 6)));
            if (r.Next(2) == 0) v = -v;
            fl.Add(v);
        }
        for (int i = 0; i < 150; i++)
        {
            float v = (float)((r.NextDouble() + 0.01) * Math.Pow(10, r.Next(-10, 12)));
            if (r.Next(2) == 0) v = -v;
            fl.Add(v);
        }
        for (int i = 0; i < 100; i++)
        {
            float a = (float)r.NextDouble(), b = (float)r.NextDouble() * 255f;
            fl.Add(a / 255f); fl.Add(a * b); fl.Add(b / 3f); fl.Add(a + 0.1f);
        }
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "float.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            foreach (float d in fl)
                foreach (string f in DoubleFmts)
                {
                    float dd = d;
                    string fmt = f;
                    string res = TryFmt(() => fmt == NONE ? dd.ToString(Inv) : dd.ToString(fmt, Inv));
                    w.WriteLine(HexF(d) + "\t" + Esc(f) + "\t" + Esc(res));
                }
        }
    }

    static readonly string[] DecFmts = {
        NONE, "0", "0.0", "0.00", "0.000", "0.###", "0.##", "#0.00", "0.0#", "#,##0", "00", "000000000", "00000.##", "000.##",
        "+00.0;-00.0", "F", "F2", "F6", "N", "N0", "N3", "P", "P1", "G", "G5", "E3", "0.00%", "#,##0.00", "0.0,,", "0.0E+00", "0.##E-0", "#.##", "#"
    };

    static List<decimal> Decimals()
    {
        List<decimal> l = new List<decimal>();
        string[] s = {
            "0", "0.0", "0.00", "-0", "-0.00", "1", "-1", "1.0", "1.00", "1000.000", "0.5", "1.5", "2.5", "-2.5", "0.0005", "1.0005", "-1.0005", "0.0004", "-0.0004",
            "123.456", "123.4560", "0.1", "0.30", "79228162514264337593543950335", "-79228162514264337593543950335", "0.0000000000000000000000000001",
            "1.0000000000000000000000000001", "3.1415926535897932384626433833", "7922816251426433759354395033.5", "100", "100.00", "0.99950", "9.995", "99.995",
            "12345678901234567890.12345678", "0.000012345", "1E+5", "1234567.891", "-0.05", "-0.005", "0.045", "0.0449999999999999999", "1.15", "1.25", "1.35"
        };
        foreach (string x in s) l.Add(decimal.Parse(x, NumberStyles.Float, Inv));
        l.Add(1m / 3m); l.Add(2m / 3m); l.Add(10m / 4m); l.Add(1m / 7m); l.Add(100m / 3m); l.Add(-1m / 3m);
        l.Add(1.10m * 1.10m); l.Add(0.1m + 0.2m); l.Add(1.50m + 1.5m); l.Add(2.0m * 3.00m); l.Add(10m / 5m); l.Add(1.0m / 8m);
        Random r = new Random(4242);
        for (int i = 0; i < 80; i++)
        {
            decimal v = (decimal)r.Next(0, 1000000) / (decimal)Math.Pow(10, r.Next(0, 7));
            if (r.Next(2) == 0) v = -v;
            l.Add(v);
        }
        return l;
    }

    static void DecimalTable(string dir)
    {
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "decimal.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            foreach (decimal d in Decimals())
                foreach (string f in DecFmts)
                {
                    decimal dd = d;
                    string fmt = f;
                    string res = TryFmt(() => fmt == NONE ? dd.ToString(Inv) : dd.ToString(fmt, Inv));
                    w.WriteLine(DS(d) + "\t" + Esc(f) + "\t" + Esc(res));
                }
        }
        // arithmetic
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "decimal_ops.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            List<decimal> ds = Decimals();
            Random r = new Random(99);
            for (int i = 0; i < 1500; i++)
            {
                decimal a = ds[r.Next(ds.Count)], b = ds[r.Next(ds.Count)];
                string[] ops = { "+", "-", "*", "/", "%" };
                foreach (string op in ops)
                {
                    decimal aa = a, bb = b;
                    string o = op;
                    string res = TryFmt(() =>
                    {
                        switch (o)
                        {
                            case "+": return (aa + bb).ToString(Inv);
                            case "-": return (aa - bb).ToString(Inv);
                            case "*": return (aa * bb).ToString(Inv);
                            case "/": return (aa / bb).ToString(Inv);
                            default: return (aa % bb).ToString(Inv);
                        }
                    });
                    w.WriteLine(op + "\t" + DS(a) + "\t" + DS(b) + "\t" + res);
                }
            }
            // Math.Round(decimal, n) both modes
            for (int i = 0; i < 400; i++)
            {
                decimal a = ds[r.Next(ds.Count)];
                int n = r.Next(0, 5);
                decimal aa = a; int nn = n;
                w.WriteLine("round\t" + DS(a) + "\t" + n + "\t" + TryFmt(() => Math.Round(aa, nn).ToString(Inv)));
                w.WriteLine("rounda\t" + DS(a) + "\t" + n + "\t" + TryFmt(() => Math.Round(aa, nn, MidpointRounding.AwayFromZero).ToString(Inv)));
            }
        }
    }

    static readonly string[] IntFmts = {
        NONE, "0", "0.0", "0.00", "000", "000000000", "00000000", "00000", "#,##0", "#", "+00;-00", "D", "D2", "D5", "X", "X2", "x", "x2", "x8", "X8", "N", "N0", "N2", "F2", "G", "P1", "E2", "0.0%", "000.##", "0,0"
    };

    static void IntTable(string dir)
    {
        long[] vals = { 0, 1, -1, 5, 9, 10, 15, 16, 99, 100, 255, 256, 1000, -1000, 12345, 1234567, 65535, 2147483647, -2147483648L, 305419896, -305419896, 4294967295L };
        long[] vals64 = { 9223372036854775807L, -9223372036854775808L, 1234567890123456L, -42 };
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "int.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            foreach (string f in IntFmts)
            {
                string fmt = f;
                foreach (long v in vals)
                {
                    long vv = v;
                    if (v >= int.MinValue && v <= int.MaxValue)
                        w.WriteLine("i32\t" + v + "\t" + Esc(f) + "\t" + Esc(TryFmt(() => fmt == NONE ? ((int)vv).ToString(Inv) : ((int)vv).ToString(fmt, Inv))));
                    if (v >= 0 && v <= uint.MaxValue)
                        w.WriteLine("u32\t" + v + "\t" + Esc(f) + "\t" + Esc(TryFmt(() => fmt == NONE ? ((uint)vv).ToString(Inv) : ((uint)vv).ToString(fmt, Inv))));
                    if (v >= 0 && v <= 255)
                        w.WriteLine("u8\t" + v + "\t" + Esc(f) + "\t" + Esc(TryFmt(() => fmt == NONE ? ((byte)vv).ToString(Inv) : ((byte)vv).ToString(fmt, Inv))));
                }
                foreach (long v in vals64)
                {
                    long vv = v;
                    w.WriteLine("i64\t" + v + "\t" + Esc(f) + "\t" + Esc(TryFmt(() => fmt == NONE ? vv.ToString(Inv) : vv.ToString(fmt, Inv))));
                }
                foreach (long v in vals)
                {
                    long vv = v;
                    w.WriteLine("i64\t" + v + "\t" + Esc(f) + "\t" + Esc(TryFmt(() => fmt == NONE ? vv.ToString(Inv) : vv.ToString(fmt, Inv))));
                }
            }
        }
    }

    static void ParseTable(string dir)
    {
        string[] inputs = {
            "0", "1", "-1", "+1", "1.5", "-1.5", "+1.5", ".5", "5.", ".", "-", "+", "", " ", "  12  ", "\t12\n", "\u000b12\u000c", "12 3", "1,234", "1,234.5", ",1", "1,", "1,,2", "1,.5", "1.2.3", "1e5", "1E5", "1e+5", "1e-5", "1e", "1e+", "1.5e3x", "e5", "1e5.5",
            "NaN", "nan", "Infinity", "-Infinity", "+Infinity", "infinity", "-infinity", "∞", "-∞", "1e308", "1e309", "-1e309", "1e-400", "1e-324", "4.9e-324", "2.4703282292062327e-324", "2.4703282292062328e-324",
            "123456789012345678901234567890", "0.1234567890123456789", "00012", "-0", "-0.0", "0.00", "1.7976931348623157e308", "1.7976931348623159e308", "3.4028235e38", "3.4028236e38", "3.40282357e38", "1e39", "1.00000017881393432617187499", "1.00000017881393432617187500", "1.00000017881393432617187501",
            "2147483647", "2147483648", "-2147483648", "-2147483649", "9223372036854775807", "9223372036854775808", "4294967295", "99999999999999999999",
            "(1)", "$1", "1$", "1-", "-1-", "- 1", "+-1", "1 -", "0x10", "1_000", "١٢", "１", "1\u0000", "\u00001", "1\u0000\u0000", " 1\u0000 ", "1.", "-.5", "+.5e1", "0e0", "0.0e5", "1,2,3,4", "12,34.56,7",
            "79228162514264337593543950335", "79228162514264337593543950336", "-79228162514264337593543950335", "7922816251426433759354395033.5", "7922816251426433759354395033.4", "0.00000000000000000000000000001", "0.00000000000000000000000000005", "0.0000000000000000000000000000500001", "0.12345678901234567890123456789", "0.99999999999999999999999999995", "1.00000000000000000000000000005",
            "-0.00000000000000000000000000000001", "0.00000000000000000000000000000001", "0.000000000000000000000000000000", "-0.0000000000000000000000000000000", "0.00000000000000000000000000000", "1.00000000000000000000000000005", "1.00000000000000000000000000015", "1.00000000000000000000000000025", "0.123456789012345678901234567891", "0.123456789012345678901234567850", "0.1234567890123456789012345678500000001", "0.00000000000000000000000000006", "0.000000000000000000000000000051", "0.0000000000000000000000000000500000000000000000000000000001", "0.00000000000000000000000000015", "0.00000000000000000000000000025", "0.0000000000000000000000000000000000000000000000000000000000000000001", "1.5000000000000000000000000000000000000000000000000000000", "12345678901234567890123456789.5", "12345678901234567890123456789.4", "1234567890123456789012345678.95", "0.00000000000000000000000000000009", "0.00000000000000000000000000000005", "0.000000000000000000000000000000999", "-0.00000000000000000000000000009", "1234567890123456789012345678.85", "123.4560", "0.10", "100", "1000.000", "  -12.5  ", "12.50 ", "00.00", "-00.00", "1e2", "1.0000000000000000000000000000000000000000001", "5000000000000000000000000000.00", "12345678901234567890123456789.0"
        };
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "parse.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            foreach (string s in inputs)
            {
                string ss = s;
                string e = Esc(s);
                w.WriteLine("double\t" + e + "\t" + TryFmt(() => { double v; return double.TryParse(ss, NumberStyles.Float | NumberStyles.AllowThousands, Inv, out v) ? Hex(v) : "FAIL"; }));
                w.WriteLine("double_default\t" + e + "\t" + TryFmt(() => Hex(double.Parse(ss, Inv))));
                w.WriteLine("float\t" + e + "\t" + TryFmt(() => { float v; return float.TryParse(ss, NumberStyles.Float | NumberStyles.AllowThousands, Inv, out v) ? HexF(v) : "FAIL"; }));
                w.WriteLine("int\t" + e + "\t" + TryFmt(() => { int v; return int.TryParse(ss, NumberStyles.Integer, Inv, out v) ? v.ToString(Inv) : "FAIL"; }));
                w.WriteLine("decimal\t" + e + "\t" + TryFmt(() => { decimal v; return decimal.TryParse(ss, NumberStyles.Number, Inv, out v) ? v.ToString(Inv) : "FAIL"; }));
            }
        }
    }

    static void CastTable(string dir)
    {
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "cast.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            double[] vs = {
                0, -0.0, 0.5, -0.5, 0.99999, -0.99999, 1.5, -1.5, 2147483646.9, 2147483647.0, 2147483647.5, 2147483647.9999, 2147483648.0, -2147483647.9, -2147483648.0, -2147483648.5, -2147483648.9, -2147483649.0, -2147483649.5,
                4294967295.0, 4294967296.0, 1e10, -1e10, 1e18, 9.2e18, 9223372036854775807.0, 9223372036854775808.0, -9223372036854775808.0, -9223372036854777856.0, 1e19, 1e300, double.NaN, double.PositiveInfinity, double.NegativeInfinity, 255.9, 256.0, -1.0, 3.99999999999999999
            };
            foreach (double v in vs)
            {
                double d = v;
                float f = (float)v;
                unchecked
                {
                    w.WriteLine("d2i\t" + Hex(d) + "\t" + (int)d);
                    w.WriteLine("d2l\t" + Hex(d) + "\t" + (long)d);
                    w.WriteLine("f2i\t" + HexF(f) + "\t" + (int)f);
                    w.WriteLine("f2l\t" + HexF(f) + "\t" + (long)f);
                }
                w.WriteLine("d2dec\t" + Hex(d) + "\t" + TryFmt(() => ((decimal)d).ToString(Inv)));
                w.WriteLine("f2dec\t" + HexF(f) + "\t" + TryFmt(() => ((decimal)f).ToString(Inv)));
                w.WriteLine("round\t" + Hex(d) + "\t" + Hex(Math.Round(d)));
            }
            Random r = new Random(5);
            for (int i = 0; i < 300; i++)
            {
                double d = (r.NextDouble() - 0.3) * Math.Pow(10, r.Next(-35, 31));
                float f = (float)d;
                w.WriteLine("d2dec\t" + Hex(d) + "\t" + TryFmt(() => ((decimal)d).ToString(Inv)));
                w.WriteLine("f2dec\t" + HexF(f) + "\t" + TryFmt(() => ((decimal)f).ToString(Inv)));
                // 2 decimals values like 12.35
                double d2 = Math.Round(d, 3);
                w.WriteLine("d2dec\t" + Hex(d2) + "\t" + TryFmt(() => ((decimal)d2).ToString(Inv)));
            }
            foreach (decimal dd in Decimals())
            {
                decimal x = dd;
                w.WriteLine("dec2d\t" + DS(dd) + "\t" + Hex((double)x));
                w.WriteLine("dec2f\t" + DS(dd) + "\t" + HexF((float)x));
            }
        }
    }

    static void TimeTable(string dir)
    {
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "timespan.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            double[] vs = { 0, 1, -1, 0.5, 1.5, -1.5, 0.0001, 0.00001, 0.016666, 1.0 / 3, 2.0 / 3, 100.1, 12345.6789, 0.001, 0.0004, 0.0005, 0.0006, -0.0005, -0.0006, 59.9999, 3600, 86400.5, 1e6, 7.123456, 0.1, 0.2, 0.3, 90, 125.75, 1e-9, 1234567.891, 33.333333, 1440.5, 24.0000001 };
            Random r = new Random(11);
            List<double> all = new List<double>(vs);
            for (int i = 0; i < 80; i++) all.Add((r.NextDouble() - 0.1) * Math.Pow(10, r.Next(-5, 6)));
            foreach (double v in all)
            {
                double d = v;
                string[] names = { "days", "hours", "minutes", "seconds", "ms" };
                for (int k = 0; k < 5; k++)
                {
                    int kk = k;
                    string res = TryFmt(() =>
                    {
                        TimeSpan t = kk == 0 ? TimeSpan.FromDays(d) : kk == 1 ? TimeSpan.FromHours(d) : kk == 2 ? TimeSpan.FromMinutes(d) : kk == 3 ? TimeSpan.FromSeconds(d) : TimeSpan.FromMilliseconds(d);
                        return t.Ticks + "";
                    });
                    w.WriteLine("from_" + names[k] + "\t" + Hex(d) + "\t" + res);
                }
            }
            // properties of tick values
            List<long> ticks = new List<long> { 0, 1, -1, 9999, 10000, 10001, 600000000, 36000000000, 864000000000, 864000000001, -864000000001, 123456789012345, -123456789012345, 9223372036854775807, -9223372036854775808, 5, 1234567, 35999999999, 3599999999, 599999999, -5, -1234567, 8640000000000, 26352000000000 };
            for (int i = 0; i < 80; i++) ticks.Add(((long)r.Next() << 20) ^ r.Next());
            for (int i = 0; i < 40; i++) ticks.Add(-(((long)r.Next() << 14) ^ r.Next()));
            foreach (long t in ticks)
            {
                TimeSpan ts = new TimeSpan(t);
                w.WriteLine("props\t" + t + "\t" + ts.Days + " " + ts.Hours + " " + ts.Minutes + " " + ts.Seconds + " " + ts.Milliseconds + " " + Hex(ts.TotalDays) + " " + Hex(ts.TotalHours) + " " + Hex(ts.TotalMinutes) + " " + Hex(ts.TotalSeconds) + " " + Hex(ts.TotalMilliseconds));
                w.WriteLine("str\t" + t + "\t" + ts.ToString());
            }
        }
    }

    static void CharTable(string dir)
    {
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "chars.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            string[] names = { "letter", "digit", "number", "white" };
            for (int k = 0; k < 4; k++)
            {
                int start = -1;
                for (int c = 0; c <= 0x10000; c++)
                {
                    bool b = false;
                    if (c < 0x10000)
                    {
                        char ch = (char)c;
                        b = k == 0 ? char.IsLetter(ch) : k == 1 ? char.IsDigit(ch) : k == 2 ? char.IsNumber(ch) : char.IsWhiteSpace(ch);
                    }
                    if (b && start < 0) start = c;
                    if (!b && start >= 0) { w.WriteLine(names[k] + "\t" + start.ToString("x") + "\t" + (c - 1).ToString("x")); start = -1; }
                }
            }
        }
    }

    static void SplitTable(string dir)
    {
        using (StreamWriter w = new StreamWriter(Path.Combine(dir, "split.tsv"), false, new UTF8Encoding(false)))
        {
            w.NewLine = "\n";
            string[] ss = { "", "a", "a,b", ",a", "a,", ",", ",,", "a,,b", "a, b ,c", "G1 X1.0 Y2.0", "  a  b ", "x\ty", "a,b,c,d,e" };
            foreach (string s in ss)
                foreach (char sep in new char[] { ',', ' ' })
                {
                    string[] parts = s.Split(sep);
                    List<string> e = new List<string>();
                    foreach (string p in parts) e.Add(Esc(p));
                    w.WriteLine(Esc(s) + "\t" + (int)sep + "\t" + parts.Length + "\t" + string.Join("|", e.ToArray()));
                }
        }
    }

    static int Main(string[] args)
    {
        string dir = args.Length > 0 ? args[0] : ".";
        Directory.CreateDirectory(dir);
        CultureInfo.DefaultThreadCurrentCulture = Inv;
        System.Threading.Thread.CurrentThread.CurrentCulture = Inv;
        DoubleTable(dir);
        FloatTable(dir);
        DecimalTable(dir);
        IntTable(dir);
        ParseTable(dir);
        CastTable(dir);
        TimeTable(dir);
        CharTable(dir);
        SplitTable(dir);
        return 0;
    }
}

// Golden generator for Hershey.CreateString. Output: header line + one line per resulting GrblCommand.Command.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

static class HersheyGolden
{
    static StringBuilder w = new StringBuilder();
    static int n = 0;

    static string Esc(string s)
    {
        var sb = new StringBuilder();
        foreach (char c in s)
        {
            if (c == '\\') sb.Append("\\\\");
            else if (c == '\t') sb.Append("\\t");
            else if (c == '\n') sb.Append("\\n");
            else if (c < 32 || c > 126) sb.Append("\\u" + ((int)c).ToString("x4"));
            else sb.Append(c);
        }
        return sb.ToString();
    }

    static string R(double d) { return d.ToString("R", CultureInfo.InvariantCulture); }

    static void Case(string text, double cx, double cy, int f, int s, bool horiz, string ton)
    {
        var l = LaserGRBL.Hershey.Hershey.CreateString(text, cx, cy, f, s, horiz, ton);
        w.Append("CASE\t").Append(Esc(text)).Append('\t').Append(R(cx)).Append('\t').Append(R(cy)).Append('\t').Append(f).Append('\t').Append(s).Append('\t').Append(horiz ? "H" : "V").Append('\t').Append(ton).Append('\t').Append(l.Count).Append('\n');
        foreach (var c in l) w.Append(c.Command).Append('\n');
        n++;
    }

    static void Main(string[] args)
    {
        string[] tons = { "M3", "M4" };
        // every printable ASCII char alone
        for (int c = 32; c <= 126; c++)
            foreach (bool h in new[] { true, false })
                foreach (var t in tons)
                    Case(((char)c).ToString(), 10, 20, 1000, 255, h, t);
        // strings
        string all = ""; for (int c = 32; c <= 126; c++) all += (char)c;
        string[] strs = { "", " ", "  ", "0123456789", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz", all, "Hello, World!", "LaserGRBL", "AV", "WAVE", "i l 1 |", "tab\there", "line\nbreak", "café", "€5", "a\U0001F600b", "é", "xé", "éx", "~~", "{}[]()<>", "@#$%^&*" };
        foreach (var s in strs)
            foreach (bool h in new[] { true, false })
                foreach (var t in tons)
                    Case(s, 0, 0, 1000, 255, h, t);
        // strings LaserGRBL uses in GenerateCuttingTest / GenerateGreyscaleTest
        var used = new List<string>();
        for (int x = 0; x < 12; x++) { used.Add("F" + (int)(1000 + x * 250)); used.Add("S" + (int)(100 + x * 75)); }
        for (int y = 0; y < 10; y++) used.Add((int)(1 + y) + "pass");
        used.Add("F1000"); used.Add("F100"); used.Add("F10000");
        used.Add("LaserGRBL cutting test"); used.Add("LaserGRBL cutting test [Plywood 3mm]");
        used.Add("LaserGRBL power/speed test"); used.Add("LaserGRBL power/speed test [Birch]");
        used.Add("S255, F1000 - F3000, 1 - F5 pass, M4");
        used.Add("S1000, F1000, 2 pass, M3");
        used.Add("S100 - S1000, F1000 - F3000, M4,  4 line/mm");
        used.Add("S1000, F1000, M3,  0.1 line/mm");
        used.Add("S1000, F1000 - F2000, M4,  10 line/mm");
        foreach (var s in used)
        {
            for (int h = 0; h < 2; h++)
                foreach (var t in tons)
                {
                    double ox = 10, oy = 10;
                    Case(s, 14 * 2 + 5 + ox, oy / 2, 1000, 255, h == 0, t);
                    Case(s, ox / 2, 14 * 3 + 5 + oy, 1500, 1000, h == 0, t);
                    Case(s, 70 / 2.0 + ox, 70 + oy + oy + oy / 2, 3000, 100, h == 0, t);
                }
        }
        // positions / numeric edge cases
        double[] pos = { 0, 1, -1, 0.5, 100, -100.25, 1.0 / 3.0, 2.0 / 3.0, 1234.56789, 1e-7, -1e-7, 123456789.123, 1e15, 1e16, 0.1, 0.7, 12.345678901234567, 99999.999999 };
        foreach (var x in pos)
            foreach (var y in pos)
                Case("A1", x, y, 500, 128, true, "M4");
        foreach (var x in pos)
            Case("Az0", x, 3.3, 500, 128, false, "M3");
        // speeds / powers
        int[] fs = { 0, 1, 100, 100000, -5 };
        int[] ss = { 0, 1, 255, 1000, -1 };
        foreach (var f in fs) foreach (var s in ss) Case("S12", 7.7, 8.8, f, s, f % 2 == 0, "M4");
        // odd laser-on strings
        foreach (var t in new[] { "M4", "M3", "m4", "M3 S100", "G1 M3", "", "M33", " M4 " })
            foreach (bool h in new[] { true, false })
                Case("Aj", 5, 5, 1000, 255, h, t);

        File.WriteAllText(args[0], w.ToString(), new UTF8Encoding(false));
        Console.WriteLine("cases: " + n);
    }
}

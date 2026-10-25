// Golden generator for lg-services: reference data written by the original code under Mono.
//   args: <outdir>
// Outputs:
//   keys.txt              code <TAB> KeysConverter text, for every Keys code 0..255 plus modifier combos
//   hotkeys_default.txt   default HotKeysManager table (sorted): action, value, keys int, KeysConverter text
//   project_sample.lps    Project.StoreSettings output (2 entries, enums/decimal/bool/int/string values)
//   project_sample.txt    description of the stored values
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Text;
using System.Windows.Forms;

static class GenServices
{
    static void Main(string[] args)
    {
        string outdir = args[0];
        Directory.CreateDirectory(outdir);
        Hotkeys(outdir);
        Project(outdir);
    }

    static string KeyText(Keys k)
    {
        return new KeysConverter().ConvertToString(k);
    }

    static void Hotkeys(string outdir)
    {
        var sb = new StringBuilder();
        for (int i = 0; i < 256; i++)
            sb.AppendLine(i + "\t" + KeyText((Keys)i));
        foreach (int i in new[] { 8, 13, 27, 32, 33, 34, 35, 36, 37, 38, 39, 40, 45, 46, 65, 112, 107, 109, 106, 111, 48, 96 })
        {
            sb.AppendLine((i | (int)Keys.Control) + "\t" + KeyText((Keys)i | Keys.Control));
            sb.AppendLine((i | (int)Keys.Shift) + "\t" + KeyText((Keys)i | Keys.Shift));
            sb.AppendLine((i | (int)Keys.Alt) + "\t" + KeyText((Keys)i | Keys.Alt));
            sb.AppendLine((i | (int)Keys.Control | (int)Keys.Shift | (int)Keys.Alt) + "\t" + KeyText((Keys)i | Keys.Control | Keys.Shift | Keys.Alt));
        }
        File.WriteAllText(Path.Combine(outdir, "keys.txt"), sb.ToString());

        var hm = new LaserGRBL.HotKeysManager();
        hm.Init(null, null, null);
        sb = new StringBuilder();
        foreach (var hk in hm)
            sb.AppendLine(hk.ActionString + "\t" + (int)hk.Action + "\t" + (int)hk.Combination + "\t" + (hk.Combination == Keys.None ? "" : KeyText(hk.Combination)));
        File.WriteAllText(Path.Combine(outdir, "hotkeys_default.txt"), sb.ToString());
    }

    static void Project(string outdir)
    {
        var asm = typeof(LaserGRBL.Settings).Assembly;
        var toolT = asm.GetType("LaserGRBL.RasterConverter.ImageProcessor+Tool");
        var dirT = asm.GetType("LaserGRBL.RasterConverter.ImageProcessor+Direction");
        var d1 = new Dictionary<string, object>(StringComparer.OrdinalIgnoreCase)
        {
            { "GrayScaleConversion.RasterConversionTool", Enum.ToObject(toolT, 2) },
            { "GrayScaleConversion.Line2LineOptions.Direction", Enum.ToObject(dirT, 4) },
            { "GrayScaleConversion.Line2LineOptions.Quality", 3.5m },
            { "GrayScaleConversion.Line2LineOptions.Preview", true },
            { "GrayScaleConversion.Parameters.Interpolation", System.Drawing.Drawing2D.InterpolationMode.HighQualityBicubic },
            { "GrayScaleConversion.Parameters.R", 100 },
            { "ImageName", "first.png" },
            { "ImageBase64", "iVBORw0KGgo=" },
        };
        var d2 = new Dictionary<string, object>(StringComparer.OrdinalIgnoreCase)
        {
            { "GrayScaleConversion.RasterConversionTool", Enum.ToObject(toolT, 0) },
            { "GrayScaleConversion.Line2LineOptions.Quality", 10.0m },
            { "GrayScaleConversion.Parameters.Threshold.Enabled", false },
            { "GrayScaleConversion.Parameters.Brightness", -7 },
            { "ImageName", "second.jpg" },
            { "ImageBase64", "/9j/4AAQ" },
        };
        Tools.Project.ProjectSettings.Add(d1);
        Tools.Project.ProjectSettings.Add(d2);
        string f = Path.Combine(outdir, "project_sample.lps");
        Tools.Project.StoreSettings(f);
        // extra shapes to learn the object id layout
        Func<string, Dictionary<string, object>> mk = (n) => new Dictionary<string, object>(StringComparer.OrdinalIgnoreCase) { { "k.tool", Enum.ToObject(toolT, 1) }, { "k.int", 5 }, { "ImageName", n }, { "ImageBase64", "QQ==" } };
        Tools.Project.ProjectSettings = new List<Dictionary<string, object>>();
        Tools.Project.StoreSettings(Path.Combine(outdir, "project_empty.lps"));
        Tools.Project.ProjectSettings.Add(mk("a.png"));
        Tools.Project.StoreSettings(Path.Combine(outdir, "project_one.lps"));
        Tools.Project.ProjectSettings = new List<Dictionary<string, object>>();
        Tools.Project.ProjectSettings.Add(mk("a.png"));
        Tools.Project.ProjectSettings.Add(mk("b.png"));
        Tools.Project.ProjectSettings.Add(mk("c.png"));
        Tools.Project.StoreSettings(Path.Combine(outdir, "project_three.lps"));
        Tools.Project.ProjectSettings = new List<Dictionary<string, object>>();
        Tools.Project.ProjectSettings.Add(d1);
        Tools.Project.ProjectSettings.Add(d2);
        var back = Tools.Project.LoadProject(f);
        var sb = new StringBuilder();
        for (int i = 0; i < back.Count; i++)
            foreach (var kv in back[i])
                sb.AppendLine(i + "\t" + kv.Key + "\t" + kv.Value.GetType().FullName + "\t" + Convert.ToString(kv.Value, System.Globalization.CultureInfo.InvariantCulture));
        File.WriteAllText(Path.Combine(outdir, "project_sample.txt"), sb.ToString());
    }
}

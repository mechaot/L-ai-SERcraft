// Golden generator for lg-base: NRBF reference streams written by the original code under Mono.
//   args: <outdir>
// Outputs:
//   settings_sample.bin     LaserGRBL.Settings.bin produced by Settings.SetObject (mixed types)
//   settings_sample.txt     key <TAB> type <TAB> value  (what was stored)
//   buttons_0.nrbf          raw BinaryFormatter stream of an empty List<CustomButton>
//   buttons_2.nrbf          2 buttons built here (no image / with image)
//   buttons_2.txt           description of the buttons
//   buttons_std_reser.zbn   StandardButtons.zbn read by Tools.Serializer and written again (compressed)
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Reflection;
using System.Text;
using System.Runtime.Serialization.Formatters.Binary;

static class GenBase
{
    static byte[] Ser(object o)
    {
        var f = new BinaryFormatter();
        f.AssemblyFormat = System.Runtime.Serialization.Formatters.FormatterAssemblyStyle.Simple;
        using (var ms = new MemoryStream()) { f.Serialize(ms, o); return ms.ToArray(); }
    }

    static void Main(string[] args)
    {
        string outdir = args[0];
        Directory.CreateDirectory(outdir);
        var asm = typeof(LaserGRBL.Settings).Assembly;

        // ---- settings
        var txt = new StringBuilder();
        Action<string, object> put = (k, v) =>
        {
            LaserGRBL.Settings.SetObject(k, v);
            txt.AppendLine(k + "\t" + v.GetType().FullName + "\t" + (v is object[] ? string.Join("|", Array.ConvertAll((object[])v, x => Convert.ToString(x, System.Globalization.CultureInfo.InvariantCulture))) : Convert.ToString(v, System.Globalization.CultureInfo.InvariantCulture)));
        };
        put("t.bool", true);
        put("t.int", -42);
        put("t.float", 1.5f);
        put("t.double", 0.1);
        put("t.decimal", 12.50m);
        put("t.string", "hello ä€");
        put("t.long", 1234567890123L);
        put("t.enum", LaserGRBL.Settings.GraphicMode.FBO);
        put("t.timespan", TimeSpan.FromSeconds(90));
        put("t.version", new Version(1, 2, 3));
        put("t.objarray", new object[] { 1, 2.5, "x", true });
        put("t.point", new Point(3, 4));
        put("t.size", new Size(5, 6));
        put("t.datetime", new DateTime(2024, 5, 6, 7, 8, 9, DateTimeKind.Utc));
        var ex = typeof(LaserGRBL.Settings).GetMethod("Exiting", BindingFlags.NonPublic | BindingFlags.Static);
        ex.Invoke(null, null);
        string appdata = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        File.Copy(Path.Combine(appdata, "LaserGRBL", "LaserGRBL.Settings.bin"), Path.Combine(outdir, "settings_sample.bin"), true);
        File.WriteAllText(Path.Combine(outdir, "settings_sample.txt"), txt.ToString());

        // ---- custom buttons
        File.WriteAllBytes(Path.Combine(outdir, "buttons_0.nrbf"), Ser(new List<LaserGRBL.CustomButton>()));
        var l = new List<LaserGRBL.CustomButton>();
        var b1 = new LaserGRBL.CustomButton();
        b1.guid = new Guid("01020304-0506-0708-090a-0b0c0d0e0f10");
        b1.GCode = "G0 X1"; b1.GCode2 = ""; b1.Caption = "A"; b1.ToolTip = "tip A";
        b1.EnableStyle = LaserGRBL.CustomButton.EnableStyles.Idle; b1.ButtonType = LaserGRBL.CustomButton.ButtonTypes.TwoStateButton;
        l.Add(b1);
        var b2 = new LaserGRBL.CustomButton();
        b2.guid = new Guid("11121314-1516-1718-191a-1b1c1d1e1f20");
        b2.Image = new Bitmap(4, 4);
        b2.GCode = "M3"; b2.GCode2 = ""; b2.Caption = ""; b2.ToolTip = "";
        l.Add(b2);
        File.WriteAllBytes(Path.Combine(outdir, "buttons_2.nrbf"), Ser(l));
        File.WriteAllText(Path.Combine(outdir, "buttons_2.txt"), "b1 no image; b2 4x4 bitmap, empty strings\n");

        // ---- reserialize standard buttons
        string std = args.Length > 1 ? args[1] : null;
        if (std != null)
        {
            var o = Tools.Serializer.ObjFromFile(std);
            Tools.Serializer.ObjToFile(o, Path.Combine(outdir, "buttons_std_reser.zbn"), Tools.Serializer.SerializationMode.Binary, null, true);
        }
        Console.WriteLine("ok");
    }
}

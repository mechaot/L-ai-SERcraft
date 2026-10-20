// Golden generator for Tools.Expression. Output: one TAB-separated line per case.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

static class ExprGolden
{
    // variable sets: 0 = none, 1 = custom-button style set
    static string[] VarSets = { "", "left=-12.5;right=100;top=47.25;bottom=0.1;width=112.5;height=47.15;jogstep=10;jogspeed=1000;WCO.X=1.5;WCO.Y=-2;MPos.X=3;$30=1000;$32=0;x=2;y=-3;z=0;t=1;e=7;a1=0.5;true_=1" };

    static string Bits(double d)
    {
        if (double.IsNaN(d)) return "NaN";
        return BitConverter.DoubleToInt64Bits(d).ToString("x16") + "/" + d.ToString("R", CultureInfo.InvariantCulture);
    }

    static string Err(Exception e)
    {
        return "ERR:" + e.GetType().Name;
    }

    static void Main(string[] args)
    {
        var exprs = new List<string>();
        // numbers & basic operators
        string[] nums = { "0", "1", "2", "3.5", "-4", "10", "0.1", "1e3", "1e-3", "2.5e+2", "100", ".5", "5.", "1,5", "007" };
        string[] ops = { "+", "-", "*", "/", "^", "==", "!=", "<", ">", "<=", ">=", "&&", "||" };
        foreach (var a in nums) foreach (var o in ops) { exprs.Add(a + o + "2"); }
        foreach (var o in ops) { exprs.Add("3 " + o + " 4"); exprs.Add("3 " + o + " -4"); exprs.Add("-3 " + o + " 4"); exprs.Add("(3" + o + "4)"); exprs.Add("0 " + o + " 0"); exprs.Add("1 " + o + " 1");}
        // precedence
        string[] prec = { "1+2*3", "1*2+3", "(1+2)*3", "2^3^2", "2*3^2", "-2^2", "10-4-3", "100/10/5", "100/10*5", "1+2==3", "1+2>=3&&2<1", "1<2&&2<3", "1<2||0", "1+1<3", "2*3>5", "1-2-3-4", "8/4/2", "2^-1", "2^(-1)", "1 - - 2", "1 + - 2", "1 - -2", "1--2", "1+-2", "1*-2", "1/-2", "-(1+2)", "-(-3)", "--3", "- - 3", "+3", "+ + 3", "1++2", "1+ +2", "2*(3+4)*5", "((1))", "(((2)))+1", "(1)(2)", "2(3)", "(2)3", "1 2", "1  2 3", "4 / 2 * 3", "1e2*2", "1e+2*2", "1e-2*2", "2e3", "2E3", "2E-3", "1e2e3", "1e", "e", "1+e", "3e+", "3e-", "1e-2-3", "1e+2+3", "5e-1*2", "2.5E+1-1" };
        exprs.AddRange(prec);
        // functions
        string[] fn = { "abs(-3)", "abs(3)", "abs(0)", "abs(-3.5)+1", "abs(1-5)", "abs (2-9)", "abs(abs(-2))", "abs", "abs 3", "abs-3", "1+abs(-2)", "2*abs(-2)", "abs(-2)*abs(3)", "sign(3)", "sign(-3)", "sign(0)", "sign -3", "sign 3", "sign(-3)+sign(3)", "sign(1-2)*5", "neg 3", "neg(3)", "neg(-3)", "neg(neg(3))", "1+neg 3", "1 neg 3", "neg", "2*neg(3)", "sign", "-sign(3)", "sign(abs(-3))", "abs(sign(-3))", "abs(-2)^2", "2^abs(-2)", "abs(2)abs(3)", "10*(abs(-1))" };
        exprs.AddRange(fn);
        // booleans / constants
        string[] bl = { "true", "false", "true&&false", "true||false", "true&&true", "false||false", "1==1", "1==2", "1!=2", "true+1", "false*5", "true==1", "!true", "!=", "==", "&&", "||", "true==true", "(1<2)==(2<3)", "1<2<3", "3>2>1", "1<=1", "2>=3", "1<=", "<1", ">1", "1><2", "1<>2", "1=<2", "1=>2", "1<<2", "1>>2", "1>=>2", "1<=<2", "1<2>3" };
        exprs.AddRange(bl);
        // variables
        string[] vr = { "left", "right", "top", "bottom", "width", "height", "left+width/2", "(left+right)/2", "top-height/2", "width*height", "$30", "$30*0.5", "$30/2", "$30 * 0.5", "$32", "$31", "[$30]", "WCO.X", "WCO.Y+1", "WCO.X*2", "MPos.X-WCO.X", "MPos.Y", "jogstep*2", "jogspeed/60", "x+y", "x*y", "x-y", "x/y", "x/z", "z/z", "y^x", "x^y", "x^0.5", "y^0.5", "z^z", "z^-1", "undefined", "undefined+1", "x+undefined", "1+2+undefined", "e", "e+1", "e*e", "t", "t+t", "a1", "a1*4", "true_", "true_+1", "designed", "sign", "xsign", "sign x", "x sign", "absx", "abs x", "negx", "x neg", "left-right", "left - right", "left - -right", "left--right", "-left", "- left", "-left+right", "-width/-height", "-x*-y", "-x^2", "(-x)^2", "x^-2", "x^ -2", "x ^ - 2", "x^(-2)", "2^x^2", "(2^x)^2", "x==2", "x!=2", "x<y", "x>y", "x<=2", "y>=-3", "x>1&&y<0", "x>5||y<0", "x>5&&y<0", "(x>1)&&(y<0)", "x*(y+z)", "x*(y+z)*t", "(x)(y)", "x(y)", "(x)y", "x y", "x  y", "x + y z", "WCO.X+WCO.Y", "WCO.Z", "wco.x", "$", "$ 30", "$30$30", "1$30", "2*$30", "$30*2", "$30e3", "x1", "1x", "x.y", ".", "..", "a.b.c", "a1+b2", "_", "__x", "a_b", "x_y" };
        exprs.AddRange(vr);
        // sci notation confusion with variables
        string[] sci = { "e-1", "e+1", "1e-1", "1e - 1", "1 e - 1", "2e-3e-4", "1.5e-3", "1.e-3", ".5e-3", "1e-3+2e+4", "3-1e-3", "3 - 1e-3", "3-1e+3", "1e+3-1e-3", "x-1e-3", "1e-3-x", "e-e", "e+e", "e-3", "3-e", "3-e-3", "2e-3-1", "2e-3 -1", "12e-12", "1e-10*1e10", "1e300*1e10", "1e-300/1e300", "1e308+1e308", "-1e308-1e308", "1e15", "1e14", "123456789012345", "1234567890123456", "12345678901234567890", "0.1+0.2", "0.1*3", "1/3", "2/3", "10/3", "1/3*3", "100/7", "1e-5", "0.00001", "0.000001", "1e-6+0", "1e-5*1", "123456789.123456789*1", "0.1+0.7", "4.35*100", "1.005*1000", "3*1.1", "9007199254740993", "9007199254740993*1" };
        exprs.AddRange(sci);
        // errors / structure
        string[] er = { "", " ", "()", "(", ")", ")(", "(()", "())", "(1", "1)", "((1)", "1+", "+", "*", "*1", "1*", "/", "1/0", "0/0", "-1/0", "x/0", "1/(1-1)", "^", "1^", "^1", "(+)", "(1+)", "(+1)", "(*1)", "1(", "1()", "()1", "abs()", "abs(", "abs)", "neg()", "sign()", "1 abs(2)", "abs(2) 1", "2 neg 3", "neg neg 3", "neg -3", "neg - 3", "-neg 3", "1-neg 3", "1 - neg 3", "{1}", "{", "}", "1{", "{1+2}", "1+{2}", "{1}+{2}", "{x}", "1,5+1", "1,000", "1,000+1", "1,00,0", ",", "1,", ",1", "a,b", "1;2", "1:2", "NaN", "Infinity", "-Infinity", "+Infinity", " NaN ", " Infinity", "infinity", "nan", "NaN+1", "Infinity-Infinity", "1/NaN", "0*Infinity", "true false", "truefalse", "false+true", "true()", "(true)", "1==", "==1", "1!=", "!=1", "1&&", "&&1", "1||", "||1", "1|2", "1&2", "1&&&2", "1|||2", "1===2", "1!==2", "1=2", "1!2", "!1", "~1", "1%2", "1\\2", "'1'", "\"1\"", "1_000", "0x10", "1f", "1d", "1L", "5 5", "5	5", "١٢٣", "1\t+\t2", " 1 + 2 ", "  1  +  2  ", "1+2 ", " 1+2", "\t1+2", "1 + 2\t" , "1+2\n", "(1+2)\n*3" };
        exprs.AddRange(er);
        // custom button style samples
        string[] cb = { "left+width/2", "right-10", "top-5", "bottom+5", "width/2", "height/2", "(right-left)/2", "WCO.X+10", "MPos.X+jogstep", "$30*0.5", "$30*0.25", "$30/4", "$30 * 0.75", "$30-1", "$30*(width/100)", "jogspeed*60", "jogstep*0.1", "left*-1", "left*(-1)", "-1*left", "(left+right)/2-WCO.X", "(top+bottom)/2-WCO.Y", "abs(left)", "abs(left-right)", "sign(left)*5", "x>0", "x>0&&y>0", "x*100/ $30", "1000", "255", "0.5", "1e3", "100*0.01", "100*(1-0.25)" };
        exprs.AddRange(cb);
        // generated random-ish combos (deterministic)
        var rnd = new Random(12345);
        string[] atoms = { "1", "2", "3", "0.5", "10", "x", "y", "-2", "left", "$30", "e", "7.25", "0", "1e2", "width" };
        string[] bin = { "+", "-", "*", "/", "^", "<", ">", "<=", ">=", "==", "!=", "&&", "||" };
        string[] un = { "", "", "", "-", "abs", "neg", "sign" };
        for (int i = 0; i < 400; i++)
        {
            Func<string> atom = () => {
                string a = atoms[rnd.Next(atoms.Length)];
                string u = un[rnd.Next(un.Length)];
                if (u == "") return a;
                if (u == "-") return "-" + a;
                return u + "(" + a + ")";
            };
            var sb = new StringBuilder();
            int n = 2 + rnd.Next(4);
            int open = 0;
            for (int k = 0; k < n; k++)
            {
                if (rnd.Next(5) == 0) { sb.Append("("); open++; }
                sb.Append(atom());
                if (open > 0 && rnd.Next(3) == 0) { sb.Append(")"); open--; }
                if (k < n - 1) sb.Append(rnd.Next(4) == 0 ? " " + bin[rnd.Next(bin.Length)] + " " : bin[rnd.Next(bin.Length)]);
            }
            while (open-- > 0) sb.Append(")");
            exprs.Add(sb.ToString());
        }

        var seen = new HashSet<string>();
        var w = new StringBuilder();
        w.Append("# idx\tvarset\texpr(escaped)\tctor\texpand\tpostfix\tevalD\tevalB\n");
        int idx = 0;
        foreach (var ex in exprs)
        {
            if (!seen.Add(ex)) continue;
            for (int vs = 0; vs < VarSets.Length; vs++)
            {
                string ctor, exp = "-", post = "-", d = "-", b = "-";
                Tools.Expression e = null;
                try { e = new Tools.Expression(ex); ctor = "OK"; }
                catch (Exception x) { ctor = Err(x); }
                try { exp = new Tools.Expression().Expand(ex); } catch (Exception x) { exp = Err(x); }
                if (e != null)
                {
                    post = e.PostFix;
                    foreach (var kv in VarSets[vs].Split(new[] { ';' }, StringSplitOptions.RemoveEmptyEntries))
                    {
                        var p = kv.Split('=');
                        e.AddSetVariable(p[0], double.Parse(p[1], CultureInfo.InvariantCulture));
                    }
                    try { d = Bits(e.EvaluateD()); } catch (Exception x) { d = Err(x); }
                    try { b = e.EvaluateB().ToString(); } catch (Exception x) { b = Err(x); }
                }
                w.Append(idx++).Append('\t').Append(vs).Append('\t').Append(Esc(ex)).Append('\t').Append(ctor).Append('\t').Append(Esc(exp)).Append('\t').Append(Esc(post)).Append('\t').Append(d).Append('\t').Append(b).Append('\n');
            }
        }
        File.WriteAllText(args[0], w.ToString(), new UTF8Encoding(false));
        Console.WriteLine("cases: " + idx);
    }

    static string Esc(string s)
    {
        var sb = new StringBuilder();
        foreach (char c in s)
        {
            if (c == '\\') sb.Append("\\\\");
            else if (c == '\t') sb.Append("\\t");
            else if (c == '\n') sb.Append("\\n");
            else if (c == '\r') sb.Append("\\r");
            else sb.Append(c);
        }
        return sb.ToString();
    }
}

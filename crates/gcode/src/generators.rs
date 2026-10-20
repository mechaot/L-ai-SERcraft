//! Port of the test-pattern generators of `GrblFile.cs`: cutting test, greyscale (power/speed)
//! test, shake test.

use crate::command::GrblCommand;
use crate::grbl_file::GrblFile;
use crate::GResult;
use lg_dotnet::f64_to_i32;

impl GrblFile {
    fn fnum(n: f64) -> String {
        Self::formatnumber(n)
    }

    /// Append hershey text as commands.
    ///
    /// `lines`: raw lines from `lg_hershey::create_string`.
    fn add_raw(&mut self, lines: Vec<String>) {
        self.list.extend(lines.iter().map(|l| GrblCommand::new(l)));
    }

    /// C# `GenerateCuttingTest`. Returns Ok(false) when the file is busy (C# returns silently).
    ///
    /// `f_col`: feed columns. `f_start`,`f_end`: feed range. `p_start`,`p_end`: passes range.
    /// `s_fixed`: S value. `f_text`,`s_text`: text feed / power. `title`: optional title.
    /// `ton`: laser-on command (M3/M4).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_cutting_test(&mut self, f_col: i32, f_start: i32, f_end: i32, p_start: i32, p_end: i32, s_fixed: i32, f_text: i32, s_text: i32, title: &str, ton: &str) -> GResult<bool> {
        if self.check_in_use() {
            return Ok(false);
        }
        let p_row = p_end - p_start + 1;
        let ox = 3.0f64;
        let oy = 3.0f64;
        let filename = "Cutting Test";
        let x_size = f_col * 14 - 4;
        let y_size = p_row * 14 - 4;

        self.rise_on_file_loading(filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        self.clear_list();
        self.range.reset_range();

        let f_delta = if f_col > 1 { (f_end - f_start) as f64 / (f_col - 1) as f64 } else { 0.0 };

        // back to origin
        self.list.push(GrblCommand::new(&format!("G0 X{} Y{} S{}", Self::fnum(ox), Self::fnum(oy), Self::fnum(s_fixed as f64))));
        self.list.push(GrblCommand::new(&format!("G1 {} F{}", ton, Self::fnum(f_start as f64))));

        for p in 0..p_row {
            let cy = oy + 14.0 * p as f64;
            for f in 0..f_col {
                let cx = ox + 14.0 * f as f64;
                for _pass in 0..(p_start + p) {
                    self.list.push(GrblCommand::new(&format!("G0 X{} Y{}", Self::fnum(cx), Self::fnum(cy))));
                    // draw rectangle, laser on
                    self.list.push(GrblCommand::new(&format!("G1 X{} F{} {}", Self::fnum(cx + 10.0), Self::fnum(f_start as f64 + f_delta * f as f64), ton)));
                    self.list.push(GrblCommand::new(&format!("G1 Y{}", Self::fnum(cy + 10.0))));
                    self.list.push(GrblCommand::new(&format!("G1 X{}", Self::fnum(cx))));
                    self.list.push(GrblCommand::new(&format!("G1 Y{}", Self::fnum(cy))));
                    self.list.push(GrblCommand::new("M5"));
                }
            }
        }

        // back to origin
        self.list.push(GrblCommand::new(&format!("G0 X{} Y{} S0", Self::fnum(ox), Self::fnum(oy))));
        self.list.push(GrblCommand::new(&format!("G1 {} F{}", ton, Self::fnum(f_text as f64))));

        for x in 0..f_col {
            let t = format!("F{}", f64_to_i32(f_start as f64 + (x as f64 * f_delta)));
            let l = lg_hershey::create_string(&t, ((x * 14) + (10 / 2)) as f64 + ox, oy / 2.0, f_text, s_text, true, "M4");
            self.add_raw(l);
        }
        for y in 0..p_row {
            let t = format!("{}pass", p_start + y);
            let l = lg_hershey::create_string(&t, ox / 2.0, ((y * 14) + (10 / 2)) as f64 + oy, f_text, s_text, false, "M4");
            self.add_raw(l);
        }

        let srange = format!("S{s_fixed}");
        let frange = if f_start != f_end { format!("F{f_start} - F{f_end}") } else { format!("F{f_end}") };
        // NOTE(upstream): "F{p_end}" in the pass range text is a typo of the original, kept
        let prange = if p_start != p_end { format!("{p_start} - F{p_end} pass") } else { format!("{p_end} pass") };

        let title = if title.is_empty() { "LaserGRBL cutting test".to_string() } else { format!("LaserGRBL cutting test [{title}]") };
        let parmessage = format!("{srange}, {frange}, {prange}, {ton}");
        let l = lg_hershey::create_string(&title, (x_size / 2) as f64 + ox, y_size as f64 + oy + oy + oy / 2.0, f_text, s_text, true, "M4");
        self.add_raw(l);
        let l = lg_hershey::create_string(&parmessage, (x_size / 2) as f64 + ox, y_size as f64 + oy + oy / 2.0, f_text, s_text, true, "M4");
        self.add_raw(l);

        // laser off
        self.list.push(GrblCommand::new("M5"));

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(true)
    }

    /// C# `GenerateGreyscaleTest` (power vs speed). Returns Ok(false) when busy.
    ///
    /// `f_row`: feed rows. `s_col`: power columns. `f_start`,`f_end`: feed range.
    /// `s_start`,`s_end`: power range. `x_size`,`y_size`: size in mm. `resolution`: lines/mm.
    /// `f_grid`,`s_grid`: grid feed / power. `title`: optional. `f_text`,`s_text`: text feed /
    /// power. `ton`: laser-on command.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_greyscale_test(&mut self, f_row: i32, s_col: i32, f_start: i32, f_end: i32, s_start: i32, s_end: i32, x_size: i32, y_size: i32, resolution: f64, f_grid: i32, s_grid: i32, title: &str, f_text: i32, s_text: i32, ton: &str) -> GResult<bool> {
        if self.check_in_use() {
            return Ok(false);
        }
        let ox = 3.0f64;
        let oy = 3.0f64;
        let filename = "PowerSpeed Test";
        self.rise_on_file_loading(filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        self.clear_list();
        self.range.reset_range();

        let f_delta = if f_row > 1 { (f_end - f_start) as f64 / (f_row - 1) as f64 } else { 0.0 };
        let s_delta = if s_col > 1 { (s_end - s_start) as f64 / (s_col - 1) as f64 } else { 0.0 };
        let x_step = x_size as f64 / s_col as f64;
        let y_step = y_size as f64 / f_row as f64;
        let filling_step = 1.0 / resolution;

        // back to origin
        self.list.push(GrblCommand::new(&format!("G0 X{} Y{} S0", Self::fnum(ox), Self::fnum(oy))));
        self.list.push(GrblCommand::new(&format!("G1 {} F{}", ton, Self::fnum(f_start as f64))));

        // draw filling
        let mut prev_f = f64::NAN;
        let mut forward = true;
        let mut y = 0.0f64;
        while y <= y_size as f64 {
            let cur_f = f_start as f64 + (f64_to_i32(y / y_step)) as f64 * f_delta;
            // NaN != x is always true
            if cur_f != prev_f {
                self.list.push(GrblCommand::new(&format!("F{}", Self::fnum(cur_f))));
                prev_f = cur_f;
            }
            self.list.push(GrblCommand::new(&format!("Y{} S0", Self::fnum(oy + y))));
            for x in 0..s_col {
                let cx = if forward { (x + 1) as f64 * x_step } else { x_size as f64 - ((x + 1) as f64 * x_step) };
                let cs = if forward { s_start as f64 + (x as f64 * s_delta) } else { s_end as f64 - (x as f64 * s_delta) };
                self.list.push(GrblCommand::new(&format!("X{} S{}", Self::fnum(ox + cx), Self::fnum(cs))));
            }
            forward = !forward;
            y += filling_step;
        }

        // back to origin
        self.list.push(GrblCommand::new(&format!("G0 X{} Y{} S0", Self::fnum(ox), Self::fnum(oy))));
        self.list.push(GrblCommand::new(&format!("G1 {} F{}", ton, Self::fnum(f_grid as f64))));

        // grid X
        forward = true;
        for y in 0..(f_row + 1) {
            let cy = y as f64 * y_step;
            self.list.push(GrblCommand::new(&format!("Y{} S0", Self::fnum(oy + cy))));
            for x in 0..(s_col + 1) {
                let cx = if forward { x as f64 * x_step } else { x_size as f64 - x as f64 * x_step };
                self.list.push(GrblCommand::new(&format!("X{} S{}", Self::fnum(ox + cx), Self::fnum(s_grid as f64))));
            }
            forward = !forward;
        }

        // back to origin
        self.list.push(GrblCommand::new(&format!("G0 X{} Y{} S0", Self::fnum(ox), Self::fnum(oy))));
        self.list.push(GrblCommand::new(&format!("G1 {} F{}", ton, Self::fnum(f_grid as f64))));

        // grid Y
        forward = true;
        for x in 0..(s_col + 1) {
            let cx = x as f64 * x_step;
            self.list.push(GrblCommand::new(&format!("X{} S0", Self::fnum(ox + cx))));
            for y in 0..(f_row + 1) {
                let cy = if forward { y as f64 * y_step } else { y_size as f64 - y as f64 * y_step };
                self.list.push(GrblCommand::new(&format!("Y{} S{}", Self::fnum(oy + cy), Self::fnum(s_grid as f64))));
            }
            forward = !forward;
        }

        for x in 0..s_col {
            let t = format!("S{}", f64_to_i32(s_start as f64 + (x as f64 * s_delta)));
            let l = lg_hershey::create_string(&t, (x as f64 * x_step) + (x_step / 2.0) + ox, oy / 2.0, f_text, s_text, true, "M4");
            self.add_raw(l);
        }
        for y in 0..f_row {
            let t = format!("F{}", f64_to_i32(f_start as f64 + (y as f64 * f_delta)));
            let l = lg_hershey::create_string(&t, ox / 2.0, (y as f64 * y_step) + (y_step / 2.0) + oy, f_text, s_text, false, "M4");
            self.add_raw(l);
        }

        let srange = if s_start != s_end { format!("S{s_start} - S{s_end}") } else { format!("S{s_end}") };
        let frange = if f_start != f_end { format!("F{f_start} - F{f_end}") } else { format!("F{f_end}") };
        let title = if title.is_empty() { "LaserGRBL power/speed test".to_string() } else { format!("LaserGRBL power/speed test [{title}]") };
        let parmessage = format!("{srange}, {frange}, {ton},  {} line/mm", Self::fnum(resolution));
        let l = lg_hershey::create_string(&title, (x_size / 2) as f64 + ox, y_size as f64 + oy + oy + oy / 2.0, f_text, s_text, true, "M4");
        self.add_raw(l);
        let l = lg_hershey::create_string(&parmessage, (x_size / 2) as f64 + ox, y_size as f64 + oy + oy / 2.0, f_text, s_text, true, "M4");
        self.add_raw(l);

        // laser off
        self.list.push(GrblCommand::new("M5"));

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(filename, elapsed);
        Ok(true)
    }

    /// C# `GenerateShakeTest`. Returns Ok(false) when busy.
    ///
    /// `axis`: "X" or "Y". `flimit`: feed limit. `axislen`: travel in mm. `cpower`: cross power.
    /// `cspeed`: cross feed.
    pub fn generate_shake_test(&mut self, axis: &str, flimit: i32, axislen: i32, cpower: i32, cspeed: i32) -> GResult<bool> {
        if self.check_in_use() {
            return Ok(false);
        }
        let filename = format!("Shake Test {axis}");
        self.rise_on_file_loading(&filename);
        let start = lg_base::tools::hires_timer::total_milliseconds();
        self.clear_list();
        self.range.reset_range();

        let mut add = |s: String| self.list.push(GrblCommand::new(&s));
        add("M5".into()); // laser OFF
        add("G1 F1000 X0 Y0 S0".into()); // origin, slowly
        add(format!("G1 F{cspeed} X7 Y10"));
        add("M4".into());
        add(format!("G1 F{cspeed} S{cpower} X13 Y10"));
        add("M5".into());
        add(format!("G1 F{cspeed} X10 Y7"));
        add("M4".into());
        add(format!("G1 F{cspeed} S{cpower} X10 Y13"));
        add("M5".into());
        add("G1 F1000 X10 Y10 S0".into()); // cross center, slowly

        self.generate_shake_test2(axis, flimit, axislen, 10, 50, 0.5);
        self.generate_shake_test2(axis, flimit, axislen, 10, 100, 2.0);
        self.generate_shake_test2(axis, flimit, axislen, 10, 200, 4.0);
        self.generate_shake_test2(axis, flimit, axislen, 10, 400, 8.0);

        let mut add = |s: String| self.list.push(GrblCommand::new(&s));
        add(format!("G1 F{cspeed} X10 Y10 S0")); // cross center, fast
        add(format!("G1 F{cspeed} X7 Y10"));
        add("M4".into());
        add(format!("G1 F{cspeed} S{cpower} X13 Y10"));
        add("M5".into());
        add(format!("G1 F{cspeed} X10 Y7"));
        add("M4".into());
        add(format!("G1 F{cspeed} S{cpower} X10 Y13"));
        add("M5".into());
        add("G1 F1000 X0 Y0 S0".into()); // origin, slowly

        self.analyze()?;
        let elapsed = lg_base::tools::hires_timer::total_milliseconds() - start;
        self.rise_on_file_loaded(&filename, elapsed);
        Ok(true)
    }

    /// C# `GenerateShakeTest2`: oscillation sweeps.
    ///
    /// `axis`: axis letter. `flimit`: feed. `axislen`: travel. `o`: offset. `trip`: block size.
    /// `step`: amplitude step.
    fn generate_shake_test2(&mut self, axis: &str, flimit: i32, axislen: i32, o: i32, trip: i32, step: f64) {
        let mut c = trip / 2;
        while c < axislen - trip / 2 {
            let mut i = 0.0f64;
            while i < (trip / 3) as f64 {
                self.list.push(GrblCommand::new(&format!("G1 F{flimit} {axis}{}", Self::fnum((o + c) as f64 + i))));
                self.list.push(GrblCommand::new(&format!("G1 F{flimit} {axis}{}", Self::fnum((o + c) as f64 - i))));
                i += step;
            }
            c += trip;
        }
    }
}

//! Port of `Tools/ExpressionEvaluator.cs`: infix expression evaluator for custom-button G-code.
//!
//! Keeps the C# quirks: string work stack (intermediate results round-trip through 15-digit
//! text), stale `dResult` for comparison operators in `evaluate_d`, odd tokenisation, etc.

// IL emulation needs !(a > b) (NaN-true), not a <= b
#![allow(clippy::neg_cmp_op_on_partial_ord)]

mod expand;
mod num;

use num::{parse_net, to_net};
use std::fmt;

pub use expand::{expand, scientific_notation_correction};

/// Errors mirroring the .NET exceptions the C# code can throw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprError {
    /// `new Exception(msg)` thrown by the validator.
    Exception(String),
    /// `InvalidOperationException` (Stack.Pop/Peek on empty stack).
    InvalidOperation,
    /// `FormatException` / `OverflowException` from `double.Parse`.
    Format,
    /// `ArgumentOutOfRangeException` from string index math.
    ArgumentOutOfRange,
    /// `NullReferenceException` (evaluate before any function was set).
    NullReference,
    /// `KeyNotFoundException` (compiled code, missing variable).
    KeyNotFound,
}

impl ExprError {
    /// Short .NET-style type name, used by the golden files.
    ///
    /// Returns the exception class name.
    pub fn kind(&self) -> &'static str {
        match self {
            ExprError::Exception(_) => "Exception",
            ExprError::InvalidOperation => "InvalidOperationException",
            ExprError::Format => "FormatException",
            ExprError::ArgumentOutOfRange => "ArgumentOutOfRangeException",
            ExprError::NullReference => "NullReferenceException",
            ExprError::KeyNotFound => "KeyNotFoundException",
        }
    }
}

impl fmt::Display for ExprError {
    /// Format as `Kind: message`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprError::Exception(m) => write!(f, "{m}"),
            e => write!(f, "{}", e.kind()),
        }
    }
}

impl std::error::Error for ExprError {}

const TRUE: f64 = 1.0;
const FALSE: f64 = 0.0;

/// One step of the "compiled" program (emulates the Reflection.Emit IL of `emitFunction`).
#[derive(Debug, Clone)]
enum Op {
    Const(f64),
    Var(String),
    Oper(String),
}

/// Evaluatable mathematical function (C# `Tools.Expression`).
#[derive(Debug, Clone, Default)]
pub struct Expression {
    in_function: String,
    post_function: String,
    // insertion-ordered like a C# Dictionary without removals
    variables: Vec<(String, f64)>,
    split_post_function: Option<Vec<String>>,
    compilecode: bool,
    dynamic_function: Option<Vec<Op>>,
}

/// Port of `IsOperator`.
///
/// * `token` - token to test.
///
/// Returns true for operators and the `sign`/`abs`/`neg` words.
pub fn is_operator(token: &str) -> bool {
    matches!(
        token,
        "+" | "-" | "*" | "/" | "^" | "&&" | "||" | "sign" | "abs" | "neg" | "==" | ">=" | "<=" | ">" | "<" | "!="
    )
}

/// Port of `IsOperand`.
///
/// * `token` - token to test.
///
/// Returns true unless token is an operator, parenthesis or brace.
pub fn is_operand(token: &str) -> bool {
    !is_operator(token) && token != "(" && token != ")" && token != "{" && token != "}"
}

/// Port of `GetPrecedence`.
///
/// * `token` - operator token.
///
/// Returns 1..=4, or 0 for unknown tokens (incl. parentheses).
pub fn get_precedence(token: &str) -> i32 {
    match token {
        "+" | "-" | "||" => 1,
        "*" | "/" | "&&" => 2,
        "==" | ">=" | "<=" | ">" | "<" | "!=" | "sign" | "^" => 3,
        "abs" | "neg" => 4,
        _ => 0,
    }
}

/// Port of `IsNumber` (`double.Parse` succeeds).
///
/// * `token` - token to test.
///
/// Returns true when .NET would parse it.
pub fn is_number(token: &str) -> bool {
    parse_net(token).is_some()
}

/// Port of `IsVariable`.
///
/// * `token` - token to test.
///
/// Returns true for operands that are neither `true`/`false` nor numbers.
pub fn is_variable(token: &str) -> bool {
    if token == "true" || token == "false" {
        return false;
    }
    is_operand(token) && !is_number(token)
}

/// Port of `Infix2Postfix` (shunting yard on the expanded text).
///
/// * `func` - raw infix text.
///
/// Returns the space-separated postfix string.
fn infix2postfix(func: &str) -> Result<String, ExprError> {
    let func = expand(func)?;
    let mut post: Vec<String> = Vec::new();
    let mut operators: Vec<String> = Vec::new();
    for token in func.split(' ') {
        if is_operand(token) {
            post.push(token.to_string());
        } else if token == "(" {
            operators.push(token.to_string());
        } else if token == ")" {
            // pop to matching "("
            let mut cur = operators.pop().ok_or(ExprError::InvalidOperation)?;
            while cur != "(" {
                post.push(cur);
                cur = operators.pop().ok_or(ExprError::InvalidOperation)?;
            }
        } else if is_operator(token) {
            // pop operators with precedence >= token
            while let Some(top) = operators.last() {
                if get_precedence(token) <= get_precedence(top) {
                    post.push(operators.pop().unwrap_or_default());
                } else {
                    break;
                }
            }
            operators.push(token.to_string());
        }
    }
    while let Some(op) = operators.pop() {
        post.push(op);
    }
    // C# builds "item + ' ' + acc" then Trim()
    let mut s = post.join(" ");
    s.push(' ');
    Ok(s.trim().to_string())
}

/// Port of `Validate`.
///
/// * `in_fix` - raw infix text.
///
/// Returns `Ok(())` or the same exception C# throws.
fn validate(in_fix: &str) -> Result<(), ExprError> {
    let clean = expand(in_fix)?;
    let func: Vec<&str> = clean.split(' ').collect();
    let ex = |m: String| Err(ExprError::Exception(m));
    let mut par_count = 0i32;
    for i in 0..func.len() {
        // abs needs "("
        if func[i] == "abs" && (i == func.len() - 1 || func[i + 1] != "(") {
            return ex(format!("Operator error! abs does not have a \"(\"{in_fix}"));
        }
        // neg must not follow an operand
        if func[i] == "neg" && i != 0 && is_operand(func[i - 1]) {
            return ex(format!("Operator error! neg used improperly.{in_fix}"));
        }
        if i > 0 && i < func.len() - 1 && (func[i] == "(" || func[i] == ")") && is_operand(func[i - 1]) && is_operand(func[i + 1]) {
            return ex(format!("Operator error!{} used improperly.{in_fix}", func[i]));
        }
        if func[i] == "(" {
            par_count += 1;
        } else if func[i] == ")" {
            par_count -= 1;
        }
        if par_count < 0 {
            return ex(format!("Parenthesis error! No matching opening parenthesis. {in_fix}"));
        }
    }
    if par_count != 0 {
        return ex(format!("Parenthesis error! No matching closeing parenthesis. {in_fix}"));
    }
    // dry run of the postfix to check operand counts
    let post = infix2postfix(in_fix)?;
    let mut depth = 0usize;
    for token in post.split(' ') {
        if is_operator(token) {
            let need = if matches!(token, "abs" | "neg" | "sign") { 1 } else { 2 };
            if depth < need {
                return ex(format!("Operator error! \"{token}\". {in_fix}"));
            }
            // pops `need`, pushes one
            depth = depth - need + 1;
        } else {
            depth += 1;
        }
    }
    Ok(())
}

impl Expression {
    /// Port of ctor `Expression(string)`.
    ///
    /// * `function` - infix text to validate and convert.
    ///
    /// Returns the expression, or the validation exception.
    pub fn new(function: &str) -> Result<Self, ExprError> {
        let mut e = Expression::default();
        e.set_function(function)?;
        Ok(e)
    }

    /// Port of property `Function` getter.
    ///
    /// Returns the raw infix text.
    pub fn function(&self) -> &str {
        &self.in_function
    }

    /// Port of property `Function` setter: validate, convert, clear variables, maybe compile.
    ///
    /// * `value` - new infix text.
    ///
    /// Returns validation / compile errors; state is untouched on validation failure.
    pub fn set_function(&mut self, value: &str) -> Result<(), ExprError> {
        validate(value)?;
        self.in_function = value.to_string();
        self.post_function = infix2postfix(&self.in_function)?;
        self.split_post_function = Some(self.post_function.split(' ').map(str::to_string).collect());
        self.clear_variables();
        if self.compilecode {
            self.compile()?;
        }
        Ok(())
    }

    /// Port of property `PostFix`.
    ///
    /// Returns the postfix text.
    pub fn post_fix(&self) -> &str {
        &self.post_function
    }

    /// Port of property `InFix`.
    ///
    /// Returns the expanded infix text.
    pub fn in_fix(&self) -> Result<String, ExprError> {
        expand(&self.in_function)
    }

    /// Port of property `Compile` getter.
    ///
    /// Returns whether the next `set_function` compiles.
    pub fn compile_enabled(&self) -> bool {
        self.compilecode
    }

    /// Port of property `Compile` setter.
    ///
    /// * `v` - new flag (takes effect on next `set_function`).
    pub fn set_compile_enabled(&mut self, v: bool) {
        self.compilecode = v;
    }

    /// Port of `Expand` (instance wrapper).
    ///
    /// * `function` - raw infix text.
    ///
    /// Returns the expanded text.
    pub fn expand(&self, function: &str) -> Result<String, ExprError> {
        expand(function)
    }

    /// Port of `AddSetVariable(string,double)`.
    ///
    /// * `name` - variable name.
    /// * `val` - value.
    pub fn add_set_variable(&mut self, name: &str, val: f64) {
        if let Some(slot) = self.variables.iter_mut().find(|(k, _)| k == name) {
            slot.1 = val;
        } else {
            self.variables.push((name.to_string(), val));
        }
    }

    /// Port of `AddSetVariable(string,bool)`.
    ///
    /// * `name` - variable name.
    /// * `val` - value, stored as 1 or 0.
    pub fn add_set_variable_bool(&mut self, name: &str, val: bool) {
        self.add_set_variable(name, if val { 1.0 } else { 0.0 });
    }

    /// Port of `ClearVariables`.
    pub fn clear_variables(&mut self) {
        self.variables.clear();
    }

    /// Port of `Clear`. NOTE(upstream): keeps the split postfix, evaluate_d still sees the old one.
    pub fn clear(&mut self) {
        self.in_function.clear();
        self.post_function.clear();
        self.variables.clear();
    }

    /// Port of property `FunctionVariables`.
    ///
    /// Returns variable-like tokens in order; errors if no function is set.
    pub fn function_variables(&self) -> Result<Vec<String>, ExprError> {
        if self.in_function.is_empty() {
            return Err(ExprError::Exception("Function does not exist".into()));
        }
        let func = expand(&self.in_function)?;
        Ok(func.split(' ').filter(|t| is_variable(t)).map(str::to_string).collect())
    }

    /// Variable lookup helper.
    fn var(&self, name: &str) -> Option<f64> {
        self.variables.iter().find(|(k, _)| k == name).map(|(_, v)| *v)
    }

    /// Port of `GetVariableValue`.
    ///
    /// * `token` - variable name.
    ///
    /// Returns the value or NaN when missing.
    pub fn get_variable_value(&self, token: &str) -> f64 {
        self.var(token).unwrap_or(f64::NAN)
    }

    /// Port of `ExistToken`.
    ///
    /// * `token` - variable name.
    ///
    /// Returns true when the variable was set.
    pub fn exist_token(&self, token: &str) -> bool {
        self.var(token).is_some()
    }

    /// Port of `ConvertString`: variable, then true/false, then `double.Parse`.
    ///
    /// * `token` - work stack item.
    ///
    /// Returns the value or `Format` error.
    fn convert_string(&self, token: &str) -> Result<f64, ExprError> {
        if let Some(v) = self.var(token) {
            return Ok(v);
        }
        match token {
            "true" => Ok(TRUE),
            "false" => Ok(FALSE),
            _ => parse_net(token).ok_or(ExprError::Format),
        }
    }

    /// Port of `EvaluateD`.
    ///
    /// Returns the numeric result or the .NET exception kind.
    pub fn evaluate_d(&self) -> Result<f64, ExprError> {
        if let Some(prog) = &self.dynamic_function {
            return self.run_compiled(prog);
        }
        let split = self.split_post_function.as_ref().ok_or(ExprError::NullReference)?;
        let mut stack: Vec<String> = Vec::new();
        let mut d_result = 0.0f64;
        for token in split {
            if is_operator(token) {
                let (d_left, d_right);
                if matches!(token.as_str(), "abs" | "neg" | "sign") {
                    let s_left = stack.pop().ok_or(ExprError::InvalidOperation)?;
                    d_left = self.convert_string(&s_left)?;
                    d_right = 0.0;
                } else {
                    let s_right = stack.pop().ok_or(ExprError::InvalidOperation)?;
                    let s_left = stack.pop().ok_or(ExprError::InvalidOperation)?;
                    d_left = self.convert_string(&s_left)?;
                    d_right = self.convert_string(&s_right)?;
                }
                // NOTE(upstream): comparison and logic ops keep the previous d_result
                match token.as_str() {
                    "+" => d_result = d_left + d_right,
                    "-" => d_result = d_left - d_right,
                    "*" => d_result = d_left * d_right,
                    "/" => d_result = if d_right == 0.0 { f64::NAN } else { d_left / d_right },
                    "^" => d_result = net_pow(d_left, d_right),
                    "sign" => d_result = if d_left >= 0.0 { 1.0 } else { -1.0 },
                    "abs" => d_result = d_left.abs(),
                    "neg" => d_result = -d_left,
                    _ => {}
                }
                stack.push(to_net(d_result));
            } else {
                stack.push(token.clone());
            }
        }
        self.convert_string(stack.last().ok_or(ExprError::InvalidOperation)?)
    }

    /// Port of `EvaluateB`.
    ///
    /// Returns true when the result equals 1; errors like `evaluate_d`.
    pub fn evaluate_b(&self) -> Result<bool, ExprError> {
        if let Some(prog) = &self.dynamic_function {
            // NOTE(upstream): C# emits a double-returning body for the bool method (invalid IL)
            return Ok(self.run_compiled(prog)? != 0.0);
        }
        let mut stack: Vec<String> = Vec::new();
        let mut s_result = String::new();
        for token in self.post_function.split(' ') {
            if is_operator(token) {
                let (d_left, d_right);
                // NOTE(upstream): "sign" is a two-operand op here
                if matches!(token, "abs" | "neg") {
                    let s_left = stack.pop().ok_or(ExprError::InvalidOperation)?;
                    d_left = self.convert_string(&s_left)?;
                    d_right = 0.0;
                } else {
                    let s_right = stack.pop().ok_or(ExprError::InvalidOperation)?;
                    let s_left = stack.pop().ok_or(ExprError::InvalidOperation)?;
                    d_left = self.convert_string(&s_left)?;
                    d_right = self.convert_string(&s_right)?;
                }
                let tf = |b: bool| if b { "true" } else { "false" }.to_string();
                match token {
                    "<=" => s_result = tf(d_left <= d_right),
                    "<" => s_result = tf(d_left < d_right),
                    ">=" => s_result = tf(d_left >= d_right),
                    ">" => s_result = tf(d_left > d_right),
                    "==" => s_result = tf(d_left == d_right),
                    "!=" => s_result = tf(d_left != d_right),
                    "||" => s_result = tf(d_right == TRUE || d_left == TRUE),
                    "&&" => s_result = tf(d_right == TRUE && d_left == TRUE),
                    "abs" => s_result = to_net(d_left.abs()),
                    "neg" => s_result = to_net(-d_left),
                    _ => {}
                }
                stack.push(s_result.clone());
            } else {
                stack.push(token.to_string());
            }
        }
        Ok(self.convert_string(stack.last().ok_or(ExprError::InvalidOperation)?)? == TRUE)
    }

    /// Port of `compile` + `emitFunction`: build the program the IL would run.
    ///
    /// Returns number-parse errors like the emitter would throw.
    fn compile(&mut self) -> Result<(), ExprError> {
        let mut prog = Vec::new();
        for token in self.post_function.split(' ') {
            if is_operator(token) {
                prog.push(Op::Oper(token.to_string()));
            } else if is_variable(token) {
                prog.push(Op::Var(token.to_string()));
            } else if token == "true" {
                prog.push(Op::Const(TRUE));
            } else if token == "false" {
                prog.push(Op::Const(FALSE));
            } else {
                prog.push(Op::Const(parse_net(token).ok_or(ExprError::Format)?));
            }
        }
        self.dynamic_function = Some(prog);
        Ok(())
    }

    /// Run the compiled program (IL semantics: comparisons yield 0/1, `<=` is `!(a>b)` etc.).
    ///
    /// * `prog` - compiled ops.
    ///
    /// Returns the top of stack; missing variables give `KeyNotFound`.
    fn run_compiled(&self, prog: &[Op]) -> Result<f64, ExprError> {
        let mut st: Vec<f64> = Vec::new();
        let pop = |st: &mut Vec<f64>| st.pop().ok_or(ExprError::InvalidOperation);
        let b2f = |b: bool| if b { 1.0 } else { 0.0 };
        for op in prog {
            match op {
                Op::Const(v) => st.push(*v),
                Op::Var(n) => st.push(self.var(n).ok_or(ExprError::KeyNotFound)?),
                Op::Oper(t) => {
                    if matches!(t.as_str(), "abs" | "neg" | "sign") {
                        let a = pop(&mut st)?;
                        st.push(match t.as_str() {
                            "abs" => a.abs(),
                            "neg" => -a,
                            _ => {
                                if a < 0.0 {
                                    -1.0
                                } else {
                                    1.0
                                }
                            }
                        });
                        continue;
                    }
                    let b = pop(&mut st)?;
                    let a = pop(&mut st)?;
                    st.push(match t.as_str() {
                        "+" => a + b,
                        "-" => a - b,
                        "*" => a * b,
                        "/" => {
                            if b == 0.0 {
                                f64::NAN
                            } else {
                                a / b
                            }
                        }
                        "^" => net_pow(a, b),
                        "<=" => b2f(!(a > b)),
                        "<" => b2f(a < b),
                        ">=" => b2f(!(a < b)),
                        ">" => b2f(a > b),
                        "==" => b2f(a == b),
                        "!=" => b2f(!(a == b)),
                        "||" => b2f(b != 0.0 || a != 0.0),
                        "&&" => b2f(b != 0.0 && a != 0.0),
                        _ => 0.0,
                    });
                }
            }
        }
        st.last().copied().ok_or(ExprError::InvalidOperation)
    }
}

/// `Math.Pow`.
///
/// * `x` - base.
/// * `y` - exponent.
///
/// Returns `x^y` (libm pow, same as Mono; .NET Fx CRT differs only for pow(1,NaN) / pow(-1,inf)).
fn net_pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}

impl fmt::Display for Expression {
    /// Port of `ToString`: function then `; k=v, k=v`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.in_function)?;
        for (i, (k, v)) in self.variables.iter().enumerate() {
            write!(f, "{}{}={}", if i == 0 { "; " } else { ", " }, k, to_net(*v))?;
        }
        Ok(())
    }
}

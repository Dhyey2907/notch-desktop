//! Quick code runner: a few lines of JavaScript (Node) or Python, run as a separate process with
//! a time limit; the last expression's value is printed, like a REPL.

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_OUTPUT: usize = 20_000;
pub const MAX_CODE: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Language {
    Javascript,
    Python,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunResult {
    pub output: String,
    pub ok: bool,
    #[ts(type = "number")]
    pub millis: u64,
}

/// Reads the code from stdin; prints the last expression's value (awaiting promises).
const NODE_WRAPPER: &str = r#"const vm=require('vm'),util=require('util');let s='';process.stdin.setEncoding('utf8');process.stdin.on('data',d=>s+=d).on('end',async()=>{try{let r=vm.runInThisContext(s,{filename:'notch'});if(r&&typeof r.then==='function')r=await r;if(r!==undefined)console.log(typeof r==='string'?r:util.inspect(r,{depth:4,colors:false}))}catch(e){console.error(e&&e.name?e.name+': '+e.message:String(e));process.exitCode=1}})"#;

const PYTHON_WRAPPER: &str = r#"import sys,ast
src=sys.stdin.read()
t=ast.parse(src,'<notch>')
last=t.body.pop() if t.body and isinstance(t.body[-1],ast.Expr) else None
g={'__name__':'__main__'}
exec(compile(t,'<notch>','exec'),g)
if last is not None:
    v=eval(compile(ast.Expression(last.value),'<notch>','eval'),g)
    if v is not None: print(v if isinstance(v,str) else repr(v))
"#;

fn command(language: Language) -> Vec<(String, Vec<String>)> {
    match language {
        Language::Javascript => vec![("node".into(), vec!["-e".into(), NODE_WRAPPER.into()])],
        // The "py" launcher first: a bare "python" may be the Microsoft Store placeholder.
        Language::Python => vec![
            ("py".into(), vec!["-3".into(), "-c".into(), PYTHON_WRAPPER.into()]),
            ("python".into(), vec!["-c".into(), PYTHON_WRAPPER.into()]),
        ],
    }
}

fn clip(mut text: String) -> String {
    if text.len() > MAX_OUTPUT {
        let mut end = MAX_OUTPUT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n… (output cut off)");
    }
    text
}

pub fn run(language: Language, code: &str) -> AppResult<RunResult> {
    if code.trim().is_empty() || code.len() > MAX_CODE {
        return Err(AppError::InvalidInput("Write some code first".into()));
    }
    let started = Instant::now();
    let mut child = None;
    for (program, args) in command(language) {
        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(std::env::temp_dir());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
        }
        if let Ok(spawned) = cmd.spawn() {
            child = Some(spawned);
            break;
        }
    }
    let mut child = child.ok_or_else(|| {
        AppError::InvalidInput(match language {
            Language::Javascript => "Node.js isn't installed (nodejs.org)".into(),
            Language::Python => "Python isn't installed (python.org)".into(),
        })
    })?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(code.as_bytes());
    }
    // Read output on threads so a chatty program can't fill the pipe and stall.
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = stdout.as_mut() {
            let _ = o.take(MAX_OUTPUT as u64 * 2).read_to_string(&mut s);
        }
        s
    });
    let err = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.take(MAX_OUTPUT as u64).read_to_string(&mut s);
        }
        s
    });
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let millis = started.elapsed().as_millis() as u64;
    let Some(status) = status else {
        return Ok(RunResult {
            output: clip(format!("{stdout}Stopped after {} seconds.", TIMEOUT.as_secs())),
            ok: false,
            millis,
        });
    };
    // Windows programs end lines with \r\n.
    let stdout = stdout.replace("\r\n", "\n");
    let stderr = stderr.replace("\r\n", "\n");
    let mut output = stdout;
    if !stderr.trim().is_empty() {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(stderr.trim_end());
    }
    Ok(RunResult {
        output: clip(output.trim_end().to_owned()),
        ok: status.success(),
        millis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs Node.js installed.
    #[test]
    #[ignore]
    fn runs_javascript() {
        let r = run(
            Language::Javascript,
            "const a = [1,2,3];\nconsole.log('hi');\na.map(x => x * 2)",
        )
        .unwrap();
        assert!(r.ok);
        assert_eq!(r.output, "hi\n[ 2, 4, 6 ]");
        let e = run(Language::Javascript, "nope()").unwrap();
        assert!(!e.ok);
        assert!(e.output.contains("ReferenceError"));
        let p = run(Language::Javascript, "Promise.resolve(42)").unwrap();
        assert_eq!(p.output, "42");
    }

    /// Needs Python installed.
    #[test]
    #[ignore]
    fn runs_python() {
        let r = run(Language::Python, "import math\nprint('hi')\nmath.sqrt(16)").unwrap();
        assert!(r.ok, "{}", r.output);
        assert_eq!(r.output, "hi\n4.0");
    }

    #[test]
    fn rejects_empty_code_and_clips_output() {
        assert!(run(Language::Python, "   ").is_err());
        let long = clip("é".repeat(MAX_OUTPUT));
        assert!(long.ends_with("(output cut off)"));
    }
}

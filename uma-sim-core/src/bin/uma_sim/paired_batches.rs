//! CLI adapter for complete, identity-matched saved batch comparisons.

use std::fs::File;
use std::io::Write;
use uma_sim_core::batch_pairs::{self, PairedBatch};

const USAGE: &str =
    "Usage: uma-sim compare-batches --input=FILE --baseline=FILE [--format=text|json]\n";

pub(super) fn run(args: &[String]) -> i32 {
    match prepare(args).and_then(|output| {
        deliver(&output).map_err(|error| format!("could not deliver comparison output: {error}"))
    }) {
        Ok(()) => 0,
        Err(error) => {
            // A second diagnostic failure must not turn an expected refusal into a panic.
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(stderr, "compare-batches: {error}");
            let _ = stderr.flush();
            1
        }
    }
}

#[cfg(unix)]
fn deliver(output: &[u8]) -> std::io::Result<()> {
    use std::os::fd::AsFd;
    // Stdout deliberately ignores EBADF. A cloned owned descriptor preserves
    // ordinary write errors, including a caller's read-only stdout descriptor.
    let stdout = std::io::stdout();
    let mut writer = File::from(stdout.as_fd().try_clone_to_owned()?);
    writer.write_all(output)?;
    writer.flush()
}

#[cfg(not(unix))]
fn deliver(output: &[u8]) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    writer.write_all(output)?;
    writer.flush()
}

fn prepare(args: &[String]) -> Result<Vec<u8>, String> {
    if args.len() == 1 && args[0] == "--help" {
        return Ok(USAGE.as_bytes().to_vec());
    }
    let mut input = None;
    let mut baseline = None;
    let mut format = None;
    for argument in args {
        let Some((name, value)) = argument.split_once('=') else {
            return Err(format!("unknown argument {argument:?}; {USAGE}"));
        };
        let slot = match name {
            "--input" => &mut input,
            "--baseline" => &mut baseline,
            "--format" => &mut format,
            _ => return Err(format!("unknown argument {argument:?}; {USAGE}")),
        };
        if slot.is_some() {
            return Err(format!("duplicate {name} argument"));
        }
        if value.is_empty() {
            return Err(format!("{name} must not be empty"));
        }
        *slot = Some(value);
    }
    let input = input.ok_or_else(|| format!("--input is required; {USAGE}"))?;
    let baseline = baseline.ok_or_else(|| format!("--baseline is required; {USAGE}"))?;
    let format = format.unwrap_or("text");
    if format != "text" && format != "json" {
        return Err(format!("--format must be text or json, got {format:?}"));
    }
    // Validate every flag before opening either explicit caller-supplied path.
    let input_file =
        File::open(input).map_err(|error| format!("could not open input {input:?}: {error}"))?;
    let input_batch = PairedBatch::from_reader(input_file, "input")?;
    let baseline_file = File::open(baseline)
        .map_err(|error| format!("could not open baseline {baseline:?}: {error}"))?;
    let baseline_batch = PairedBatch::from_reader(baseline_file, "baseline")?;
    let comparison = batch_pairs::compare(&input_batch, &baseline_batch)?;
    let mut output = if format == "json" {
        serde_json::to_vec_pretty(&comparison)
            .map_err(|error| format!("could not serialize comparison: {error}"))?
    } else {
        return Ok(batch_pairs::render_text(&comparison).into_bytes());
    };
    output.push(b'\n');
    Ok(output)
}

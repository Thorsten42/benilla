use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next() else {
        return Err("usage: pfui-audit <pfUI source folder> [--data <WoW Data folder>] [--flavor vanilla|turtle] [--with <addon folder>]... [--eval <Lua file>]".into());
    };
    let mut data = None;
    let mut turtle = false;
    let mut preload = Vec::new();
    let mut probe = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--with" => preload.push(PathBuf::from(args.next().ok_or("--with needs a folder")?)),
            "--eval" => {
                let path = args.next().ok_or("--eval needs a Lua file")?;
                probe = Some(std::fs::read_to_string(path)?);
            }
            "--data" => data = Some(PathBuf::from(args.next().ok_or("--data needs a folder")?)),
            "--flavor" => {
                turtle = match args.next().as_deref() {
                    Some("vanilla") => false,
                    Some("turtle") => true,
                    _ => return Err("--flavor must be vanilla or turtle".into()),
                }
            }
            _ => return Err("unknown argument".into()),
        }
    }
    let report = benilla_pfui::audit::inspect_probed(
        &PathBuf::from(root),
        data.as_deref(),
        turtle,
        &preload,
        probe.as_deref(),
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    // The result is a measurement. Missing syntax/APIs and a dirty stock boot must never read as
    // a successful addon check in CI; gameplay_verified is separately always false.
    if !report.source_errors.is_empty()
        || !report.compat_syntax_errors.is_empty()
        || !report.stock_ui_errors.is_empty()
        || !report.bootstrap_errors.is_empty()
        || !report.preload_errors.is_empty()
        || !report.session_errors.is_empty()
        || report
            .requirements
            .iter()
            .any(|entry| !entry.available_in_compat_vm)
    {
        std::process::exit(1);
    }
    Ok(())
}

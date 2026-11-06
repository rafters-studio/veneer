// The late initialisation is what keeps the @property defaults string alive as long
// as the sheet borrowing beside it; clippy's suggestion does not borrow-check.
#![allow(clippy::needless_late_init, clippy::format_in_format_args)]

mod css;
mod email;
mod rafter;

use std::process::ExitCode;
use std::time::Instant;

fn read_rafter(path: &str) -> Result<rafter::RafterFile, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&src).map_err(|e| format!("{path}: {e}"))
}

fn arg<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("classes") => {
            let mut all = Vec::new();
            for p in &args[1..] {
                all.extend(rafter::all_classes(&read_rafter(p)?));
            }
            all.sort();
            all.dedup();
            println!("{}", all.join("\n"));
            Ok(())
        }
        // render <file.rafter> --css <compiled.css> --out <dir>
        Some("render") => {
            let path = args
                .get(1)
                .ok_or("render <file.rafter> --css <compiled.css> --out <dir>")?;
            let css_path = arg(&args, "--css").ok_or("--css <compiled.css>")?;
            let out = arg(&args, "--out").unwrap_or(".");
            let file = read_rafter(path)?;
            let src = std::fs::read_to_string(css_path).map_err(|e| format!("{css_path}: {e}"))?;
            // Declared first so it outlives the sheet that borrows alongside it.
            let defaults_src: String;
            let sheet = css::parse(&src)?;
            defaults_src = css::property_defaults(&sheet);
            let defaults = css::parse(&defaults_src)?;
            let mut index = css::Sheet::index(&sheet, &defaults);
            let mut em = email::Emitter::new(&mut index);
            let html = em.render(&file);
            let stem = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("email");
            std::fs::write(format!("{out}/{stem}.html"), &html).map_err(|e| e.to_string())?;
            let report = serde_json::to_string_pretty(&em.report).map_err(|e| e.to_string())?;
            std::fs::write(format!("{out}/{stem}.report.json"), report)
                .map_err(|e| e.to_string())?;
            eprintln!(
                "{stem}: {} bytes, {} inlined, {} in <style>, {} dropped",
                html.len(),
                em.report.inlined.len(),
                em.report.in_style.len(),
                em.report.dropped.len()
            );
            Ok(())
        }
        // reference <file.rafter> --css-href <href> --out <dir>: the class-based page, light and dark.
        Some("reference") => {
            let path = args
                .get(1)
                .ok_or("reference <file.rafter> --css-href <href> --out <dir>")?;
            let href = arg(&args, "--css-href").ok_or("--css-href")?;
            let out = arg(&args, "--out").unwrap_or(".");
            let file = read_rafter(path)?;
            let stem = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("email");
            for (dark, suffix) in [(false, "light"), (true, "dark")] {
                std::fs::write(
                    format!("{out}/{stem}.reference.{suffix}.html"),
                    rafter::reference(&file, href, dark),
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        // bench <compiled.css> <n> <file.rafter>...
        Some("bench") => {
            let css_path = args
                .get(1)
                .ok_or("bench <compiled.css> <n> <file.rafter>...")?;
            let n: usize = args.get(2).and_then(|n| n.parse().ok()).ok_or("n")?;
            let files: Vec<rafter::RafterFile> = args[3..]
                .iter()
                .map(|p| read_rafter(p))
                .collect::<Result<_, _>>()?;
            let src = std::fs::read_to_string(css_path).map_err(|e| e.to_string())?;
            let mut load = Vec::new();
            let mut per: Vec<Vec<f64>> = vec![Vec::new(); files.len()];
            for _ in 0..n {
                let t = Instant::now();
                let defaults_src: String;
                let sheet = css::parse(&src)?;
                defaults_src = css::property_defaults(&sheet);
                let defaults = css::parse(&defaults_src)?;
                let mut index = css::Sheet::index(&sheet, &defaults);
                load.push(t.elapsed().as_secs_f64() * 1000.0);
                for (i, f) in files.iter().enumerate() {
                    let t = Instant::now();
                    let mut em = email::Emitter::new(&mut index);
                    let html = em.render(f);
                    per[i].push(t.elapsed().as_secs_f64() * 1000.0);
                    std::hint::black_box(html);
                }
            }
            let stat = |v: &mut Vec<f64>| {
                v.sort_by(f64::total_cmp);
                (v[v.len() / 2], v[v.len() - 1])
            };
            let (m, x) = stat(&mut load);
            println!(
                "css load (parse + index), {} bytes: median {m:.3} ms, max {x:.3} ms",
                src.len()
            );
            for (i, f) in files.iter().enumerate() {
                let (m, x) = stat(&mut per[i]);
                println!("render {}: median {m:.3} ms, max {x:.3} ms", f.manifest.id);
            }
            Ok(())
        }
        _ => Err("usage: t20-to-email classes|render|bench".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

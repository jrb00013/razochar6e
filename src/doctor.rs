use crate::config;
use crate::probe::run_probe;

pub fn run() -> i32 {
    let report = run_probe();
    let mut issues = 0usize;

    println!("razochar6e doctor\n");

    if report.wsl {
        println!("[ok] WSL detected — use `razochar6e wsl` for host battery control");
    }

    if report.batteries.is_empty() {
        println!("[warn] No batteries under /sys/class/power_supply");
        issues += 1;
    }

    for b in &report.batteries {
        if b.virtual_battery {
            println!(
                "[warn] {} is virtual ({:?}) — limits may not affect physical pack",
                b.name, b.model
            );
            issues += 1;
        } else {
            println!("[ok] {} physical battery detected", b.name);
        }
    }

    let any_backend = report.backends.iter().any(|b| b.available);
    if any_backend {
        println!("[ok] At least one charge-limit backend is available");
        for b in report.backends.iter().filter(|b| b.available) {
            println!("     → {} ({})", b.name, b.id);
            if !b.supports_start_threshold {
                println!("       note: start threshold not supported — end-only on this path");
            }
        }
    } else {
        println!("[fail] No charge-limit backend available");
        issues += 1;
    }

    if let Some(path) = config::config_path() {
        if path.exists() {
            match config::load() {
                Ok(c) => println!(
                    "[ok] Config {} — start={}% end={}%",
                    path.display(),
                    c.start,
                    c.end
                ),
                Err(e) => {
                    println!("[fail] Config {} invalid: {e}", path.display());
                    issues += 1;
                }
            }
        } else {
            println!(
                "[info] No config at {} — run `razochar6e config init`",
                path.display()
            );
        }
    }

    for n in &report.notes {
        println!("[info] {n}");
    }

    println!();
    if issues == 0 {
        println!("All checks passed.");
        0
    } else {
        println!("{issues} issue(s) reported. Run `razochar6e probe` for details.");
        1
    }
}

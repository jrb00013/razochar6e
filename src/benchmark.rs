//! Charge-band benchmarking with logistic regression.
//!
//! Fits a small logistic model on synthetic usage samples, scores candidate
//! (start, end) bands for P(acceptable day), and projects annual kWh / $ vs
//! floating at 100% and vs the current config.

use crate::config::{self, AppConfig};
use crate::error::{RazError, RazResult};

const DEFAULT_RATE: f64 = 0.16;
const DEFAULT_CAPACITY_WH: f64 = 90.0;
const DEFAULT_DAILY_WH: f64 = 40.0;
const DEFAULT_HOURS_AWAY: f64 = 2.0;

#[derive(Debug, Clone)]
pub struct BenchmarkOpts {
    pub rate_per_kwh: f64,
    pub capacity_wh: f64,
    pub daily_wh: f64,
    pub hours_away: f64,
    pub samples: usize,
    pub apply: bool,
}

impl Default for BenchmarkOpts {
    fn default() -> Self {
        Self {
            rate_per_kwh: DEFAULT_RATE,
            capacity_wh: DEFAULT_CAPACITY_WH,
            daily_wh: DEFAULT_DAILY_WH,
            hours_away: DEFAULT_HOURS_AWAY,
            samples: 400,
            apply: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BandScore {
    pub start: u8,
    pub end: u8,
    pub p_acceptable: f64,
    pub annual_kwh: f64,
    pub annual_cost: f64,
    pub savings_vs_100: f64,
}

#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub coefficients: [f64; 4], // bias, midpoint, width, headroom
    pub recommended: BandScore,
    pub current: Option<BandScore>,
    pub float_100_cost: f64,
    pub candidates_scored: usize,
}

/// Logistic sigmoid.
fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        let e = (-z).exp();
        1.0 / (1.0 + e)
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// Features: [1, midpoint/100, width/100, headroom/100]
fn features(start: u8, end: u8, capacity_wh: f64, need_wh: f64) -> [f64; 4] {
    let start_f = f64::from(start);
    let end_f = f64::from(end);
    let mid = (start_f + end_f) / 2.0;
    let width = end_f - start_f;
    let usable = (end_f - start_f) / 100.0 * capacity_wh;
    let headroom = ((usable - need_wh) / capacity_wh * 100.0).clamp(-50.0, 100.0);
    [1.0, mid / 100.0, width / 100.0, headroom / 100.0]
}

fn predict(w: &[f64; 4], x: &[f64; 4]) -> f64 {
    sigmoid(w[0] * x[0] + w[1] * x[1] + w[2] * x[2] + w[3] * x[3])
}

/// Deterministic LCG for reproducible synthetic samples (no rand dep).
fn next_u32(state: &mut u64) -> u32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*state >> 32) as u32
}

fn next_f64(state: &mut u64) -> f64 {
    next_u32(state) as f64 / f64::from(u32::MAX)
}

/// Build synthetic labeled rows: y=1 if band covers randomized daily need with margin.
fn synthetic_dataset(
    samples: usize,
    capacity_wh: f64,
    daily_wh: f64,
    hours_away: f64,
) -> Vec<([f64; 4], f64)> {
    let mut rng = 0xC0FFEE_u64;
    let mut rows = Vec::with_capacity(samples);
    let base_need = daily_wh * (hours_away / 24.0).clamp(0.05, 1.0) * 8.0; // scale: away-window draw

    for _ in 0..samples {
        let start = 5 + (next_u32(&mut rng) % 46) as u8; // 5..50
        let end = (start + 15 + (next_u32(&mut rng) % 56) as u8).min(100); // width 15..70
        let need_jitter = base_need * (0.5 + next_f64(&mut rng)); // 0.5x..1.5x
        let x = features(start, end, capacity_wh, need_jitter);
        let usable = (f64::from(end) - f64::from(start)) / 100.0 * capacity_wh;
        // High midpoint (calendar aging / float waste) hurts label even if usable is ok.
        let mid = (f64::from(start) + f64::from(end)) / 2.0;
        let aging_penalty = ((mid - 55.0) / 40.0).clamp(0.0, 1.0);
        let enough = usable >= need_jitter * 1.05;
        let y = if enough && aging_penalty < 0.85 {
            1.0
        } else if enough && next_f64(&mut rng) > aging_penalty {
            1.0
        } else {
            0.0
        };
        rows.push((x, y));
    }
    rows
}

/// Batch GD logistic regression with L2.
fn fit_logistic(rows: &[([f64; 4], f64)], epochs: usize, lr: f64, l2: f64) -> [f64; 4] {
    let mut w = [0.0_f64; 4];
    let n = rows.len() as f64;
    if n == 0.0 {
        return w;
    }
    for _ in 0..epochs {
        let mut grad = [0.0_f64; 4];
        for (x, y) in rows {
            let p = predict(&w, x);
            let err = p - y;
            for j in 0..4 {
                grad[j] += err * x[j];
            }
        }
        for j in 0..4 {
            let penalty = if j == 0 { 0.0 } else { l2 * w[j] };
            w[j] -= lr * (grad[j] / n + penalty);
        }
    }
    w
}

/// Rough annual AC energy for holding a band: cycle losses + float at end%.
fn annual_kwh(start: u8, end: u8, capacity_wh: f64, daily_wh: f64) -> f64 {
    let depth = (f64::from(end) - f64::from(start)) / 100.0;
    let cycles_per_day = if depth > 0.01 {
        (daily_wh / (depth * capacity_wh)).clamp(0.1, 4.0)
    } else {
        1.0
    };
    let cycle_wh_day = cycles_per_day * depth * capacity_wh * 1.10;
    let float_wh_day = (f64::from(end) / 100.0).powi(3) * 12.0;
    let mid = (f64::from(start) + f64::from(end)) / 2.0;
    let aging_wh_day = ((mid - 50.0).max(0.0) / 50.0).powi(2) * 8.0;
    (cycle_wh_day + float_wh_day + aging_wh_day) * 365.0 / 1000.0
}

fn score_band(w: &[f64; 4], start: u8, end: u8, opts: &BenchmarkOpts) -> BandScore {
    let need = opts.daily_wh * (opts.hours_away / 24.0).clamp(0.05, 1.0) * 8.0;
    let x = features(start, end, opts.capacity_wh, need);
    let p = predict(w, &x);
    let kwh = annual_kwh(start, end, opts.capacity_wh, opts.daily_wh);
    let cost = kwh * opts.rate_per_kwh;
    BandScore {
        start,
        end,
        p_acceptable: p,
        annual_kwh: kwh,
        annual_cost: cost,
        savings_vs_100: 0.0, // filled by caller
    }
}

fn utility(b: &BandScore, float_cost: f64) -> f64 {
    let savings = (float_cost - b.annual_cost).max(0.0);
    let mid = (f64::from(b.start) + f64::from(b.end)) / 2.0;
    let mid_score = 1.0 - ((mid - 50.0).abs() / 50.0).clamp(0.0, 1.0);
    b.p_acceptable * 0.45
        + (savings / float_cost.max(0.01)).clamp(0.0, 1.0) * 0.40
        + mid_score * 0.15
}

pub fn run_benchmark(opts: BenchmarkOpts) -> RazResult<BenchmarkReport> {
    if opts.rate_per_kwh <= 0.0 || opts.capacity_wh <= 0.0 {
        return Err(RazError::InvalidThreshold(
            "rate and capacity-wh must be positive".into(),
        ));
    }
    if opts.samples < 50 {
        return Err(RazError::InvalidThreshold("samples must be >= 50".into()));
    }

    let rows = synthetic_dataset(
        opts.samples,
        opts.capacity_wh,
        opts.daily_wh,
        opts.hours_away,
    );
    let w = fit_logistic(&rows, 800, 0.35, 0.02);

    let float_100 = annual_kwh(0, 100, opts.capacity_wh, opts.daily_wh) * opts.rate_per_kwh;

    let mut best: Option<BandScore> = None;
    let mut best_u = f64::NEG_INFINITY;
    let mut n = 0usize;
    for start in (15..=40).step_by(5) {
        for end in ((start + 25).max(55)..=85).step_by(5) {
            let mut s = score_band(&w, start as u8, end as u8, &opts);
            s.savings_vs_100 = float_100 - s.annual_cost;
            let u = utility(&s, float_100);
            n += 1;
            if u > best_u {
                best_u = u;
                best = Some(s);
            }
        }
    }
    let recommended = best.ok_or_else(|| RazError::Backend {
        backend: "benchmark".into(),
        message: "no candidate bands".into(),
    })?;

    let cfg = config::load().unwrap_or_default();
    let mut current = None;
    if cfg.start < cfg.end {
        let mut s = score_band(&w, cfg.start, cfg.end, &opts);
        s.savings_vs_100 = float_100 - s.annual_cost;
        current = Some(s);
    }

    if opts.apply {
        let path = config::save(&AppConfig {
            start: recommended.start,
            end: recommended.end,
            backend: cfg.backend,
            kasa_host: cfg.kasa_host,
        })?;
        println!(
            "Applied recommended band {}–{}% to {}",
            recommended.start,
            recommended.end,
            path.display()
        );
    }

    Ok(BenchmarkReport {
        coefficients: w,
        recommended,
        current,
        float_100_cost: float_100,
        candidates_scored: n,
    })
}

pub fn print_report(r: &BenchmarkReport, opts: &BenchmarkOpts) {
    println!("razochar6e benchmark — logistic charge-band model");
    println!(
        "assumptions: ${:.4}/kWh, battery {:.0} Wh, daily draw {:.0} Wh, {:.1} h/day away, n={}",
        opts.rate_per_kwh, opts.capacity_wh, opts.daily_wh, opts.hours_away, opts.samples
    );
    println!(
        "model weights: bias={:.3} midpoint={:.3} width={:.3} headroom={:.3}",
        r.coefficients[0], r.coefficients[1], r.coefficients[2], r.coefficients[3]
    );
    println!("candidates scored: {}", r.candidates_scored);
    println!();
    println!(
        "always-100% float (projected): ${:.2}/yr ({:.1} kWh-eq heuristic)",
        r.float_100_cost,
        r.float_100_cost / opts.rate_per_kwh.max(1e-9)
    );
    if let Some(c) = &r.current {
        println!(
            "current config {}–{}%: P(ok)={:.1}%  ${:.2}/yr  save vs 100%: ${:.2}/yr",
            c.start,
            c.end,
            c.p_acceptable * 100.0,
            c.annual_cost,
            c.savings_vs_100
        );
    }
    let rec = &r.recommended;
    println!(
        "recommended {}–{}%: P(ok)={:.1}%  ${:.2}/yr ({:.1} kWh)  save vs 100%: ${:.2}/yr",
        rec.start,
        rec.end,
        rec.p_acceptable * 100.0,
        rec.annual_cost,
        rec.annual_kwh,
        rec.savings_vs_100
    );
    if let Some(c) = &r.current {
        let delta = c.annual_cost - rec.annual_cost;
        println!(
            "vs current config: {:+.2}/yr projected ({})",
            delta,
            if delta > 0.05 {
                "recommended cheaper"
            } else if delta < -0.05 {
                "current cheaper on $; check P(ok)"
            } else {
                "similar cost"
            }
        );
    }
    println!();
    println!(
        "note: projections are model heuristics for relative ranking, not utility-bill guarantees."
    );
    println!("tip: `razochar6e benchmark --apply` writes the recommended band to config.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_bounds() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-9);
        assert!(sigmoid(20.0) > 0.99);
        assert!(sigmoid(-20.0) < 0.01);
    }

    #[test]
    fn fit_separates_roughly() {
        let rows = synthetic_dataset(300, 90.0, 40.0, 2.0);
        let w = fit_logistic(&rows, 600, 0.35, 0.02);
        let good = features(20, 80, 90.0, 25.0);
        let bad_narrow = features(70, 85, 90.0, 40.0);
        assert!(
            predict(&w, &good) > predict(&w, &bad_narrow),
            "20-80 should score above a high narrow band"
        );
    }

    #[test]
    fn benchmark_returns_sane_band() {
        let r = run_benchmark(BenchmarkOpts {
            samples: 200,
            ..BenchmarkOpts::default()
        })
        .unwrap();
        assert!(r.recommended.start < r.recommended.end);
        assert!(r.recommended.end <= 90);
        assert!(r.recommended.start >= 10);
        assert!(r.float_100_cost > 0.0);
    }
}

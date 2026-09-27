# Design: `razochar6e benchmark` — optimal charge band via logistic regression

**Goal:** Recommend `start`/`end` charge percentages that balance electricity cost, calendar aging at high SoC, and risk of running out of battery — using a fitted logistic regression over candidate bands.

## Inputs (CLI / defaults)

- `--rate` $/kWh (default 0.16)
- `--capacity-wh` battery energy (default 90 Wh laptop-class; overridable)
- `--daily-wh` estimated daily draw from the pack when cycling (default 40)
- `--hours-away` hours/day the machine must run off AC (default 2)
- `--samples` synthetic usage samples for the fit (default 400)

## Model

1D/2D logistic regression (pure Rust, no ML crate):

- Features per candidate band `(start, end)`: midpoint SoC, band width, headroom above daily need.
- Labels: synthetic “acceptable day” outcomes (1 = enough energy for `hours-away` without deep discharge pain; 0 = stranded or overcharged float waste).
- Fit with batch gradient descent + L2; sigmoid probability = P(acceptable | band).

## Outputs

- Recommended `start`/`end`
- Projected annual kWh and $ vs always-100% float and vs current config
- Model coefficients + brief interpretation
- Optional `--apply` to write recommendation into config (thresholds only; does not touch Kasa)

## Out of scope

- Live meter telemetry, utility API, neural nets, cloud training.

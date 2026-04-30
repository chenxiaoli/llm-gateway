pub const UNITS_PER_USD: i64 = 100_000_000; // 8 decimal places
pub const BPS_FACTOR: i64 = 10_000;          // markup_ratio as basis points

pub fn usd_to_units(usd: f64) -> i64 {
    (usd * UNITS_PER_USD as f64).round() as i64
}

pub fn units_to_usd(units: i64) -> f64 {
    units as f64 / UNITS_PER_USD as f64
}

pub fn ratio_to_bps(ratio: f64) -> i64 {
    (ratio * BPS_FACTOR as f64).round() as i64
}

pub fn bps_to_ratio(bps: i64) -> f64 {
    bps as f64 / BPS_FACTOR as f64
}

pub fn opt_usd_to_units(usd: Option<f64>) -> Option<i64> {
    usd.map(usd_to_units)
}

pub fn opt_units_to_usd(units: Option<i64>) -> Option<f64> {
    units.map(units_to_usd)
}

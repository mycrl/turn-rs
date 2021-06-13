use anyhow::Result;

pub fn short_time(time: &str) -> Result<f64> {
    let (data, last) = time.split_at(time.len() - 1);
    Ok(match last {
        "d" => data.parse::<f64>()? * 86400.0,
        "h" => data.parse::<f64>()? * 3600.0,
        "m" => data.parse::<f64>()? * 60.0,
        "s" => data.parse::<f64>()?,
        _ => time.parse::<f64>()?
    })
}

pub fn placeholder(source: &str) -> Option<&str> {
    if source != "-" {
        Some(source)
    } else {
        None
    }
}

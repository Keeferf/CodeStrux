use crate::hardware::types::CpuInfo;

/// "Intel(R) Core(TM) i9-10900K CPU @ 3.70GHz" → "Intel i9-10900K"
/// "AMD Ryzen 9 5900X 12-Core Processor"         → "AMD Ryzen 9 5900X"
fn shorten_cpu_name(raw: &str) -> String {
    let s = raw
        .replace("(R)", "")
        .replace("(TM)", "");

    // Strip ordinal generation prefix: "12th Gen", "13th Gen", "3rd Gen", etc.
    let words: Vec<&str> = s.split_whitespace().collect();
    let s = match words.as_slice() {
        [gen, tag, rest @ ..] if
            tag.eq_ignore_ascii_case("gen") &&
            (gen.ends_with("th") || gen.ends_with("st") || gen.ends_with("nd") || gen.ends_with("rd")) =>
            rest.join(" "),
        _ => words.join(" "),
    };

    let s = s
        .find(" CPU @")
        .or_else(|| s.find(" @"))
        .map(|i| s[..i].to_string())
        .unwrap_or(s);

    let noise = ["Processor", "CPU"];
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| !noise.contains(w) && !w.ends_with("-Core") && !w.ends_with("-core"))
        .collect();

    // Intel CPUs include the redundant word "Core" between the brand and model
    // number (e.g. "Intel Core i9"); strip it for a cleaner display name.
    let words = if words.first().copied() == Some("Intel") {
        words.into_iter().filter(|w| *w != "Core").collect::<Vec<_>>()
    } else {
        words
    };

    let result = words.join(" ");
    if result.is_empty() { raw.trim().to_string() } else { result }
}

pub fn get_cpu_info(sys: &sysinfo::System) -> CpuInfo {
    let cpus = sys.cpus();
    let raw_cpu_name = cpus
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    CpuInfo {
        name: shorten_cpu_name(&raw_cpu_name),
        cores: cpus.len(),
    }
}
use std::env;

use rug::Float;
use serde::Serialize;
use tabled::{Table, Tabled, settings::Style};

fn collision_probability(n: u128, space: u128, precision: u32) -> Float {
    let lg_s = Float::with_val(precision, space + 1).ln_gamma();
    let lg_sn = Float::with_val(precision, space - n + 1).ln_gamma();
    let n_f = Float::with_val(precision, n);
    let ln_space = Float::with_val(precision, space).ln();

    let log_p_no_collision = lg_s - lg_sn - n_f * ln_space;
    let p_no_collision = log_p_no_collision.exp();
    Float::with_val(precision, 1) - p_no_collision
}

fn format_with_commas(n: u128) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

struct Group {
    pub spec: String,
    pub expanded: String,
    pub positions: u32,
}

#[derive(Serialize)]
struct JsonSpace {
    pub formula: String,
    pub size: String,
    pub groups: Vec<JsonGroup>,
}

#[derive(Serialize)]
struct JsonGroup {
    pub spec: String,
    pub positions: u32,
    pub chars: String,
}

#[derive(Serialize)]
struct JsonRow {
    pub n: String,
    pub probability: f64,
    pub percentage: f64,
    pub unique_chance: f64,
    pub retries: f64,
}

#[derive(Serialize)]
struct JsonOutput {
    pub space: JsonSpace,
    pub results: Vec<JsonRow>,
}

fn expand_chars(spec: &str) -> String {
    let chars: Vec<char> = spec.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i] <= chars[i + 2] {
            for c in chars[i]..=chars[i + 2] {
                result.push(c);
            }
            i += 3;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

fn parse_space(spec: &str) -> Result<(u128, String, Vec<Group>), String> {
    let mut space: u128 = 1;
    let mut formula = Vec::new();
    let mut groups = Vec::new();

    if spec.is_empty() {
        return Err("space spec is empty".into());
    }

    for group in spec.split(';') {
        if group.is_empty() {
            return Err("empty group in spec (double semicolon?)".into());
        }

        let (chars, positions) = match group.rsplit_once('|') {
            Some((c, n)) => {
                let pos = n
                    .parse::<u32>()
                    .map_err(|_| format!("invalid position count '{}' in group '{}'", n, group))?;
                if pos == 0 {
                    return Err(format!("position count cannot be 0 in group '{}'", group));
                }
                (c, pos)
            }
            None => (group, 1),
        };

        let expanded = expand_chars(chars);
        if expanded.is_empty() {
            return Err(format!("empty character set in group '{}'", group));
        }

        let base = expanded.len() as u128;
        let group_size = base.checked_pow(positions).ok_or_else(|| {
            format!(
                "space overflow: {}^{} exceeds u128 max ({}). Try fewer positions.",
                base,
                positions,
                format_with_commas(u128::MAX)
            )
        })?;
        space = space.checked_mul(group_size).ok_or_else(|| {
            format!(
                "total space overflow at group '{}'. Combined space exceeds u128 max ({}).",
                group,
                format_with_commas(u128::MAX)
            )
        })?;

        formula.push(format!("{}^{}", base, positions));
        groups.push(Group {
            spec: group.to_string(),
            expanded,
            positions,
        });
    }

    Ok((space, formula.join(" * "), groups))
}

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "n")]
    pub n: String,
    #[tabled(rename = "P(collision)")]
    pub probability: String,
    #[tabled(rename = "Odds")]
    pub odds: String,
    #[tabled(rename = "Unique on 1st try")]
    pub unique_chance: String,
    #[tabled(rename = "Avg retries")]
    pub retries: String,
}

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let json_output = raw_args.iter().any(|a| a == "--json");
    let args: Vec<String> = raw_args.into_iter().filter(|a| a != "--json").collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: {} [--json] '<space_spec>' <n1> [n2] [n3] ...",
            args[0]
        );
        eprintln!("  space_spec: 'chars|count;chars|count;...'");
        eprintln!("  Example: {} 'abcdefg|2;12345|3;!@#' 1000 5000", args[0]);
        std::process::exit(1);
    }

    let (space, formula, groups) = match parse_space(&args[1]) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut tests = Vec::new();
    for s in &args[2..] {
        let n = match s.parse::<u128>() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: '{}' is not a valid number", s);
                std::process::exit(1);
            }
        };
        if n == 0 {
            eprintln!("Error: n must be greater than 0");
            std::process::exit(1);
        }
        if n >= space {
            eprintln!(
                "Error: n={} must be less than space={}",
                format_with_commas(n),
                format_with_commas(space)
            );
            std::process::exit(1);
        }
        tests.push(n);
    }

    let precision = 256;

    if json_output {
        let json_groups: Vec<JsonGroup> = groups
            .iter()
            .map(|g| JsonGroup {
                spec: g.spec.clone(),
                positions: g.positions,
                chars: g.expanded.clone(),
            })
            .collect();

        let results: Vec<JsonRow> = tests
            .iter()
            .map(|&n| {
                let p = collision_probability(n, space, precision);
                let v = p.to_f64();
                let space_f = Float::with_val(precision, space);
                let remaining = Float::with_val(precision, space - n);
                let uc = (remaining.clone() / &space_f * 100u32).to_f64();
                let rt = (space_f / remaining).to_f64();

                JsonRow {
                    n: n.to_string(),
                    probability: v,
                    percentage: v * 100.0,
                    unique_chance: uc,
                    retries: rt,
                }
            })
            .collect();

        let output = JsonOutput {
            space: JsonSpace {
                formula: formula.clone(),
                size: space.to_string(),
                groups: json_groups,
            },
            results,
        };

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        return;
    }

    println!(
        "Space: {} = {} possible IDs",
        formula,
        format_with_commas(space)
    );
    println!("Spec:");
    for g in &groups {
        let chars_label = if g.positions == 1 {
            "character"
        } else {
            "characters"
        };
        println!(
            "    - {} = {} {} in \"{}\"",
            g.spec, g.positions, chars_label, g.expanded
        );
    }
    println!();

    let rows: Vec<Row> = tests
        .iter()
        .map(|&n| {
            let p = collision_probability(n, space, precision);
            let v = p.to_f64();

            let probability = if v >= 1.0 {
                String::from("~= 100%")
            } else {
                format!("{:.2e} ({:.4}%)", v, v * 100.0)
            };

            let odds = if v >= 1.0 {
                String::from("1 in 1")
            } else if v <= 0.0 {
                String::from("never")
            } else {
                let inv = 1.0 / v;
                if inv >= 10.0 {
                    format!("1 in {}", format_with_commas(inv as u128))
                } else {
                    format!("1 in {:.1}", inv)
                }
            };

            let space_f = Float::with_val(precision, space);
            let remaining = Float::with_val(precision, space - n);
            let uc = (remaining.clone() / &space_f * 100u32).to_f64();
            let rt = (space_f / remaining).to_f64();

            Row {
                n: format_with_commas(n),
                probability,
                odds,
                unique_chance: format!("{:.4}%", uc),
                retries: format!("{:.4}", rt),
            }
        })
        .collect();

    let table = Table::new(&rows).with(Style::rounded()).to_string();
    println!("{table}");
}

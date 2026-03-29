use std::env;

use rand::seq::IndexedRandom;
use rug::{Complete, Float, Integer};
use serde::Serialize;
use tabled::{Table, Tabled, settings::Style};

fn collision_probability(n: u128, space: &Integer, precision: u32) -> Float {
    let lg_s = Float::with_val(precision, Integer::from(space + 1u32)).ln_gamma();
    let lg_sn = Float::with_val(precision, Integer::from(space - n) + 1u32).ln_gamma();
    let n_f = Float::with_val(precision, n);
    let ln_space = Float::with_val(precision, space).ln();

    let log_p_no_collision = lg_s - lg_sn - n_f * ln_space;
    let p_no_collision = log_p_no_collision.exp();
    Float::with_val(precision, 1) - p_no_collision
}

fn format_with_commas(n: impl std::fmt::Display) -> String {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
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

fn generate_example_ids(groups: &[Group], count: usize) -> Vec<String> {
    let mut rng = rand::rng();
    let charsets: Vec<Vec<char>> = groups.iter().map(|g| g.expanded.chars().collect()).collect();
    (0..count)
        .map(|_| {
            let mut id = String::new();
            for (g, chars) in groups.iter().zip(&charsets) {
                for _ in 0..g.positions {
                    id.push(*chars.choose(&mut rng).unwrap());
                }
            }
            id
        })
        .collect()
}

fn parse_space(spec: &str) -> Result<(Integer, String, Vec<Group>), String> {
    let mut space = Integer::from(1u32);
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

        let base = expanded.len() as u32;
        let group_size = Integer::u_pow_u(base, positions).complete();
        space *= group_size;

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
    let space_flag = raw_args
        .iter()
        .find(|a| a == &"--space" || a.starts_with("--space="))
        .cloned();
    let sets_flag = raw_args
        .iter()
        .find(|a| a.starts_with("--sets="))
        .cloned();

    if raw_args.iter().any(|a| a == "--sets") {
        eprintln!("Error: --sets requires a value, e.g. --sets=5");
        std::process::exit(1);
    }

    let num_sets: Option<usize> = sets_flag.map(|f| {
        let val = f.strip_prefix("--sets=").unwrap();
        let n = val.parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Error: invalid --sets value '{}'", val);
            std::process::exit(1);
        });
        if n == 0 {
            eprintln!("Error: --sets must be at least 1");
            std::process::exit(1);
        }
        n.min(10)
    });

    let args: Vec<String> = raw_args
        .into_iter()
        .filter(|a| {
            a != "--json"
                && a != "--space"
                && !a.starts_with("--space=")
                && !a.starts_with("--sets=")
        })
        .collect();

    let min_args = if space_flag.is_some() { 2 } else { 3 };
    if args.len() < min_args {
        eprintln!(
            "Usage: {} [--json] [--space[=pretty]] [--sets=N] '<space_spec>' [n1] [n2] ...",
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

    if let Some(flag) = &space_flag {
        let pretty = match flag.split_once('=') {
            Some((_, "pretty")) => true,
            Some((_, v)) => {
                eprintln!("Error: unknown --space value '{}'", v);
                std::process::exit(1);
            }
            None => false,
        };

        if json_output {
            println!("{{\"space\":\"{}\"}}", space);
        } else if pretty {
            println!("{}", format_with_commas(&space));
        } else {
            println!("{}", space);
        }
        return;
    }

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
        if space <= n {
            eprintln!(
                "Error: n={} must be less than space={}",
                format_with_commas(n),
                format_with_commas(&space)
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
                let p = collision_probability(n, &space, precision);
                let v = p.to_f64();
                let space_f = Float::with_val(precision, &space);
                let remaining = Float::with_val(precision, Integer::from(&space - n));
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

        let examples = num_sets.map(|count| generate_example_ids(&groups, count));

        let output = JsonOutput {
            space: JsonSpace {
                formula: formula.clone(),
                size: space.to_string(),
                groups: json_groups,
            },
            examples,
            results,
        };

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        return;
    }

    println!(
        "Space: {} = {} possible IDs",
        formula,
        format_with_commas(&space)
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

    if let Some(count) = num_sets {
        println!();
        println!("Example IDs:");
        for ex in generate_example_ids(&groups, count) {
            println!("  {}", ex);
        }
    }
    println!();

    let rows: Vec<Row> = tests
        .iter()
        .map(|&n| {
            let p = collision_probability(n, &space, precision);
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

            let space_f = Float::with_val(precision, &space);
            let remaining = Float::with_val(precision, Integer::from(&space - n));
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

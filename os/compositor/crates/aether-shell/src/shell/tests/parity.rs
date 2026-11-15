use super::*;
use crate::palette::palette_score;

#[derive(serde::Deserialize)]
struct Fixture {
    modes: Vec<ModeCase>,
    ranks: Vec<RankCase>,
}

#[derive(serde::Deserialize)]
struct ModeCase {
    input: String,
    mode: String,
    term: String,
}

#[derive(serde::Deserialize)]
struct RankCase {
    term: String,
    title: String,
    rank: Option<u8>,
}

fn fixture() -> Fixture {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../packages/palette/fixtures/parity.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("parity fixture")).unwrap()
}

#[test]
fn prefix_modes_match_typescript() {
    for case in fixture().modes {
        let (mode, term) = parse_palette_query(&case.input);
        let mode = match mode {
            PaletteMode::All => "all",
            PaletteMode::Command => "command",
            PaletteMode::Agent => "agent",
            PaletteMode::File => "file",
            PaletteMode::Setting => "setting",
        };
        assert_eq!(mode, case.mode, "input {}", case.input);
        assert_eq!(term, case.term, "input {}", case.input);
    }
}

#[test]
fn rank_buckets_match_typescript() {
    for case in fixture().ranks {
        assert_eq!(
            palette_score(&case.term.to_lowercase(), &case.title),
            case.rank,
            "{} / {}",
            case.term,
            case.title
        );
    }
}

fn test_shell() -> Shell {
    Shell::new(Size::new(800, 600), LayoutConfig::default(), 120)
}

fn files(count: usize) -> Vec<PaletteFile> {
    (0..count)
        .map(|index| PaletteFile {
            name: format!("file-{index:05}.txt"),
            detail: "file".to_string(),
            path: format!("/tmp/file-{index:05}.txt"),
        })
        .collect()
}

#[test]
fn palette_10k_results_stay_bounded() {
    let mut shell = test_shell();
    shell.set_files(files(10_000));
    shell.palette.query = "/file".to_string();
    let entries = shell.palette_entries();
    assert!(!entries.is_empty());
    assert!(entries.len() <= 8);
    assert_eq!(shell.palette_mode(), PaletteMode::File);
}

#[test]
#[ignore = "perf: run with --release --ignored and record p95"]
fn palette_10k_p95_bench() {
    let mut shell = test_shell();
    shell.set_files(files(10_000));
    shell.palette.query = "file".to_string();
    let mut samples = Vec::new();
    for _ in 0..80 {
        let started = std::time::Instant::now();
        let entries = shell.palette_entries();
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
        assert!(!entries.is_empty());
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = samples[(samples.len() - 1) * 95 / 100];
    println!("palette_10k_p95_ms={p95:.3}");
    assert!(p95 < 10.0, "p95 {p95}ms exceeds 10ms");
}

use mox::ui::picker::PickerState;
use std::hint::black_box;
use std::time::Instant;
fn main() {
    let labels = (0..10_000)
        .map(|i| format!("workspace/{i:05}/src/main.rs"))
        .collect::<Vec<_>>();
    let mut state = PickerState::default();
    let start = Instant::now();
    for query in ["", "main", "999", "workspace/0/src", "missing"] {
        state.query = query.into();
        for _ in 0..20 {
            state.filter(
                labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| (i, label.as_str())),
            );
            black_box(&state.indices);
        }
    }
    println!(
        "picker_10000_100_filters_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let lines=(0..2000).map(|i|format!("row{i} https://example.com/{i} src/main.rs:10:2 deadbeef1234 127.0.0.1 ubuntu:24.04 pods/api #81a1c1")).collect::<Vec<_>>();
    let start = Instant::now();
    let tokens = mox::extract::scanner::scan_lines(&lines, "%0");
    println!(
        "scan_2000_lines_ms={:.3} tokens={}",
        start.elapsed().as_secs_f64() * 1000.0,
        tokens.len()
    );
    #[cfg(target_os = "linux")]
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(peak) = status.lines().find(|line| line.starts_with("VmHWM:"))
    {
        println!("{peak}");
    }
}

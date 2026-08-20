//! Microbenchmark for `Buffer::set`, the renderer's hottest write path.
//! Exists to decide #161: whether a real bounds check in `index` costs
//! anything measurable. `benches/render.rs` cannot answer that — it
//! builds its diffs outside the timed loop.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crossterm::style::Color;
use ttui::buffer::{Buffer, Cell};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;

fn painted_cell(symbol: char) -> Cell {
    Cell {
        symbol,
        fg: Color::Rgb {
            r: 200,
            g: 180,
            b: 40,
        },
        bg: Color::Reset,
        alpha: 1.0,
        ..Default::default()
    }
}

fn bench_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_set");

    // A full paint pass: every cell written once, in row-major order.
    group.bench_function("full_paint", |b| {
        let mut buf = Buffer::new(WIDTH, HEIGHT);
        let cell = painted_cell('#');
        b.iter(|| {
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    buf.set(black_box(x), black_box(y), cell.clone());
                }
            }
            black_box(&buf);
        })
    });

    // A single hot cell, to isolate per-call overhead from the loop.
    group.bench_function("single_cell", |b| {
        let mut buf = Buffer::new(WIDTH, HEIGHT);
        let cell = painted_cell('*');
        b.iter(|| {
            buf.set(black_box(10), black_box(10), cell.clone());
            black_box(&buf);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_set);
criterion_main!(benches);

//! Benchmarks `ttui::terminal::render_diff` against the pre-coalescing
//! encoder on representative diff profiles. Quantifies the byte-
//! generation savings Rev B's validation plan asked to measure; the
//! additional per-frame syscall savings from `BufWriter` are separate
//! and not captured here (flushing a `Vec<u8>` is a no-op).

use std::io::{self, Write};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::{cursor, queue};

use ttui::buffer::{diff, Buffer, Cell, CellDiff, CellStyle};
use ttui::terminal::render_diff;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 40;

/// The shipped encoder before coalescing: full MoveTo + SGR reset +
/// both colors for every cell. Kept here as the before/after baseline.
fn render_diff_naive(writer: &mut impl Write, diffs: &[CellDiff]) -> io::Result<()> {
    for d in diffs {
        let attr = if d.cell.style.bold {
            Attribute::Bold
        } else {
            Attribute::Reset
        };
        queue!(
            writer,
            cursor::MoveTo(d.x, d.y),
            SetAttribute(Attribute::Reset),
            SetAttribute(attr),
            SetForegroundColor(d.cell.fg),
            SetBackgroundColor(d.cell.bg),
            Print(d.cell.symbol),
        )?;
    }
    Ok(())
}

fn themed(symbol: char) -> Cell {
    Cell {
        symbol,
        fg: Color::Green,
        bg: Color::Reset,
        style: CellStyle { bold: false },
    }
}

/// Every cell changes — many long contiguous same-styled runs (rows).
fn full_frame() -> Vec<CellDiff> {
    let prev = Buffer::new(WIDTH, HEIGHT);
    let mut next = Buffer::new(WIDTH, HEIGHT);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            next.set(x, y, themed('#'));
        }
    }
    diff(&prev, &next)
}

/// ~1% of cells change at scattered, mostly non-contiguous positions.
fn sparse_scatter() -> Vec<CellDiff> {
    let prev = Buffer::new(WIDTH, HEIGHT);
    let mut next = Buffer::new(WIDTH, HEIGHT);
    let mut i: u32 = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            // Deterministic scatter: hit roughly every 97th cell.
            if i.is_multiple_of(97) {
                next.set(x, y, themed('*'));
            }
            i += 1;
        }
    }
    diff(&prev, &next)
}

/// A dense rectangular region changes — long contiguous runs, one
/// shared style: the case coalescing helps most.
fn dense_region() -> Vec<CellDiff> {
    let prev = Buffer::new(WIDTH, HEIGHT);
    let mut next = Buffer::new(WIDTH, HEIGHT);
    for y in 10..30 {
        for x in 20..100 {
            next.set(x, y, themed('▓'));
        }
    }
    diff(&prev, &next)
}

fn bench_render(c: &mut Criterion) {
    let profiles = [
        ("full_frame", full_frame()),
        ("sparse_scatter", sparse_scatter()),
        ("dense_region", dense_region()),
    ];

    for (name, diffs) in &profiles {
        let mut buf: Vec<u8> = Vec::with_capacity(diffs.len() * 24);
        let mut group = c.benchmark_group(*name);
        group.bench_function("coalesced", |b| {
            b.iter(|| {
                buf.clear();
                render_diff(&mut buf, black_box(diffs)).unwrap();
                black_box(&buf);
            })
        });
        group.bench_function("naive", |b| {
            b.iter(|| {
                buf.clear();
                render_diff_naive(&mut buf, black_box(diffs)).unwrap();
                black_box(&buf);
            })
        });
        group.finish();
    }
}

criterion_group!(benches, bench_render);
criterion_main!(benches);

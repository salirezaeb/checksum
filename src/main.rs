use plotters::prelude::*;
use std::error::Error;
use std::fs;

use digest::Digest;
use md5::Context;
use sha1::Sha1;
use sha2::Sha256;

const NUM_BLOCKS: usize = 10000;
const BLOCK_ROWS: usize = 8;
const BLOCK_COLS: usize = 16;
const BLOCK_SIZE: usize = BLOCK_ROWS * BLOCK_COLS;

fn checksum_sum(block: &[u8]) -> u8 {
    block.iter().fold(0u16, |acc, &v| acc + v as u16) as u8
}

fn checksum_sub(block: &[u8]) -> u8 {
    let mut acc: i16 = block[0] as i16;
    for &v in &block[1..] {
        acc -= v as i16;
    }
    (acc & 0xFF) as u8
}

fn checksum_mul(block: &[u8]) -> u8 {
    let mut acc: u16 = 1;
    for &v in block {
        acc = (acc * v as u16) % 256;
    }
    acc as u8
}

fn checksum_and(block: &[u8]) -> u8 {
    block.iter().fold(0xFF, |acc, &v| acc & v)
}

fn checksum_or(block: &[u8]) -> u8 {
    block.iter().fold(0x00, |acc, &v| acc | v)
}

fn checksum_xor(block: &[u8]) -> u8 {
    block.iter().fold(0x00, |acc, &v| acc ^ v)
}

fn checksum_xnor(block: &[u8]) -> u8 {
    !checksum_xor(block)
}

fn checksum_f(block: &[u8]) -> u8 {
    let s = checksum_sum(block);
    let x = checksum_xor(block);
    let a = checksum_and(block);
    let o = checksum_or(block);
    (s ^ x ^ (a.wrapping_add(o))) & 0xFF
}

fn save_hist(name: &str, hist: &[u32; 256]) -> Result<(), Box<dyn Error>> {
    let filename = format!("{}.png", name);
    let root = BitMapBackend::new(&filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max = *hist.iter().max().unwrap_or(&1);

    let mut chart = ChartBuilder::on(&root)
        .caption(format!("Histogram for {}", name), ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0u32..256u32, 0u32..max)?;

    chart.configure_mesh().draw()?;

    chart.draw_series((0u32..256u32).map(|x| {
        let h = hist[x as usize];
        Rectangle::new([(x, 0), (x + 1, h)], BLUE.filled())
    }))?;

    root.present()?;
    Ok(())
}

fn build_block(data: &[u8], block_idx: usize) -> Vec<u8> {
    let data_len = data.len();
    let mut block = vec![0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        let idx = (block_idx * BLOCK_SIZE + i) % data_len;
        block[i] = data[idx];
    }
    block
}

fn flip_bit(buf: &mut [u8], bitpos: usize) {
    let byte = bitpos / 8;
    let bit = bitpos % 8;
    buf[byte] ^= 1u8 << bit;
}

fn detection_coverage_adjacent_2bit<F>(data: &[u8], checksum: F) -> f64
where
    F: Fn(&[u8]) -> u8,
{
    let total_bits = BLOCK_SIZE * 8;
    let mut detected: u64 = 0;
    let mut total: u64 = 0;

    for block_idx in 0..NUM_BLOCKS {
        let block = build_block(data, block_idx);
        let c = checksum(&block);

        let mut corrupted = block.clone();
        for p in 0..(total_bits - 1) {
            flip_bit(&mut corrupted, p);
            flip_bit(&mut corrupted, p + 1);

            let c2 = checksum(&corrupted);
            if c2 != c {
                detected += 1;
            }
            total += 1;

            flip_bit(&mut corrupted, p);
            flip_bit(&mut corrupted, p + 1);
        }
    }

    (detected as f64) * 100.0 / (total as f64)
}

fn save_coverages_csv(filename: &str, rows: &[(&str, f64)]) -> Result<(), Box<dyn Error>> {
    let mut out = String::from("algorithm,coverage_percent\n");
    for (name, cov) in rows {
        out.push_str(&format!("{},{}\n", name, cov));
    }
    fs::write(filename, out)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let data = fs::read("sample.png")?;
    let data_len = data.len();

    if data_len < BLOCK_SIZE {
        return Err("sample.png is too small".into());
    }

    let mut hist_sum = [0u32; 256];
    let mut hist_sub = [0u32; 256];
    let mut hist_mul = [0u32; 256];
    let mut hist_and = [0u32; 256];
    let mut hist_or = [0u32; 256];
    let mut hist_xor = [0u32; 256];
    let mut hist_xnor = [0u32; 256];
    let mut hist_f = [0u32; 256];

    let mut hist_sha1 = [0u32; 256];
    let mut hist_md5 = [0u32; 256];
    let mut hist_sha256 = [0u32; 256];

    for block_idx in 0..NUM_BLOCKS {
        let block = build_block(&data, block_idx);

        hist_sum[checksum_sum(&block) as usize] += 1;
        hist_sub[checksum_sub(&block) as usize] += 1;
        hist_mul[checksum_mul(&block) as usize] += 1;
        hist_and[checksum_and(&block) as usize] += 1;
        hist_or[checksum_or(&block) as usize] += 1;
        hist_xor[checksum_xor(&block) as usize] += 1;
        hist_xnor[checksum_xnor(&block) as usize] += 1;
        hist_f[checksum_f(&block) as usize] += 1;

        let sha1_byte = {
            let mut h = Sha1::new();
            h.update(&block);
            h.finalize()[0]
        };

        let md5_byte = {
            let mut h = Context::new();
            h.consume(&block);
            h.compute()[0]
        };

        let sha256_byte = {
            let mut h = Sha256::new();
            h.update(&block);
            h.finalize()[0]
        };

        hist_sha1[sha1_byte as usize] += 1;
        hist_md5[md5_byte as usize] += 1;
        hist_sha256[sha256_byte as usize] += 1;
    }

    save_hist("Sum", &hist_sum)?;
    save_hist("Subtract", &hist_sub)?;
    save_hist("Multiply", &hist_mul)?;
    save_hist("AND", &hist_and)?;
    save_hist("OR", &hist_or)?;
    save_hist("XOR", &hist_xor)?;
    save_hist("XNOR", &hist_xnor)?;
    save_hist("F", &hist_f)?;

    save_hist("SHA1", &hist_sha1)?;
    save_hist("MD5", &hist_md5)?;
    save_hist("SHA256", &hist_sha256)?;

    let rows = [
        ("Sum", detection_coverage_adjacent_2bit(&data, checksum_sum)),
        ("Subtract", detection_coverage_adjacent_2bit(&data, checksum_sub)),
        ("Multiply", detection_coverage_adjacent_2bit(&data, checksum_mul)),
        ("AND", detection_coverage_adjacent_2bit(&data, checksum_and)),
        ("OR", detection_coverage_adjacent_2bit(&data, checksum_or)),
        ("XOR", detection_coverage_adjacent_2bit(&data, checksum_xor)),
        ("XNOR", detection_coverage_adjacent_2bit(&data, checksum_xnor)),
        ("F", detection_coverage_adjacent_2bit(&data, checksum_f)),
    ];

    for (name, cov) in &rows {
        println!("Coverage {:<8}: {:.6}%", name, cov);
    }

    save_coverages_csv("coverage_adjacent_2bit.csv", &rows)?;

    Ok(())
}

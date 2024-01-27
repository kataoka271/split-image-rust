use std::path::PathBuf;

use clap::Parser;
use image::{io::Reader, DynamicImage};

#[derive(Debug, Parser)]
struct Cli {
    #[clap(required = true)]
    input_path: Vec<PathBuf>,
    #[clap(long = "output", short = 'o', default_value = "output")]
    output_dir: PathBuf,
    #[clap(long = "height", short = 'H', default_value = "2000")]
    height: u32,
    #[clap(long = "margin", short = 'm', default_value = "0")]
    margin: u32,
}

// 空白判定する高さ
const BLANK_HEIGHT: usize = 20;
// 空白判定する幅 (横幅に対する割合)
const BLANK_WIDTH: Option<(f32, f32)> = Some((0.0, 0.75));
// 空白判定する分散しきい値
const BLANK_VAR_THRESHOLD: f32 = 100.0;
// 分割画像の最小の高さ
const MIN_HEIGHT: u32 = 1000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    println!("{:?}", cli);

    if !&cli.output_dir.exists() {
        std::fs::create_dir(&cli.output_dir)?;
    }
    for file in std::fs::read_dir(&cli.output_dir)? {
        let path = file?.path();
        if path.extension().map_or(false, |e| e == "png") {
            std::fs::remove_file(path)?;
        }
    }

    for (file_no, file) in cli.input_path.iter().enumerate() {
        let im = Reader::open(file)?.decode()?;

        let (_means, vars): (Vec<_>, Vec<_>) = get_mean_var(&im, BLANK_WIDTH).into_iter().unzip();
        let vars = rolling(&vars, BLANK_HEIGHT);
        let mut im_no = 0;
        let mut y_start = 0;

        while y_start < im.height() {
            let mut y_end = u32::min(y_start + cli.height, im.height());
            while y_end - y_start > MIN_HEIGHT {
                if (y_end as usize) < vars.len() && vars[y_end as usize] < BLANK_VAR_THRESHOLD {
                    break;
                }
                y_end -= 1;
            }
            let filename = &format!("output/{:02}-{:02}.png", file_no, im_no);
            im.crop_imm(0, y_start, im.width(), y_end - y_start + cli.margin)
                .save(filename)?;
            println!("{}: {}-{}", filename, y_start, y_end);
            im_no += 1;
            y_start = y_end;
        }
    }

    Ok(())
}

fn rolling(v: &Vec<f32>, n: usize) -> Vec<f32> {
    (0..v.len() - n)
        .map(|i| v[i..i + n].iter().sum::<f32>() / n as f32)
        .collect()
}

fn get_mean_var(im: &DynamicImage, range: Option<(f32, f32)>) -> Vec<(f32, f32)> {
    let range: (usize, usize) = range.map_or((0, im.width() as usize), |(fm, to)| {
        (
            (f32::max(fm, 0.0) * im.width() as f32) as usize,
            (f32::min(to, 1.0) * im.width() as f32) as usize,
        )
    });
    im.to_rgb8()
        .rows()
        .map(|row| {
            let pixels: Vec<_> = row.skip(range.0).take(range.1).collect();

            let r: Vec<f32> = pixels.iter().map(|pixel| pixel.0[0] as f32).collect();
            let g: Vec<f32> = pixels.iter().map(|pixel| pixel.0[1] as f32).collect();
            let b: Vec<f32> = pixels.iter().map(|pixel| pixel.0[2] as f32).collect();

            let n = r.len() as f32;
            let mean_r = r.iter().sum::<f32>() / n;
            let mean_g = g.iter().sum::<f32>() / n;
            let mean_b = b.iter().sum::<f32>() / n;

            let var_r = r.iter().map(|&i| (i - mean_r).powi(2)).sum::<f32>();
            let var_g = g.iter().map(|&i| (i - mean_g).powi(2)).sum::<f32>();
            let var_b = b.iter().map(|&i| (i - mean_b).powi(2)).sum::<f32>();

            let mean = mean_r + mean_g + mean_b;
            let var = var_r + var_g + var_b;

            (mean, var)
        })
        .collect()
}

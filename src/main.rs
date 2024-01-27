use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use image::{io::Reader, DynamicImage};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FileExt {
    Jpg,
    Png,
}

#[derive(Debug, Parser)]
struct Cli {
    #[clap(required = true)]
    input_path: Vec<PathBuf>,

    #[clap(long = "output", short = 'o', default_value = "output")]
    output_dir: PathBuf,

    #[clap(long = "margin", short = 'm', default_value = "0")]
    margin: u32,

    // 分割画像の最大の高さ
    #[clap(
        long = "max-height",
        short = 'H',
        default_value = "2000",
        help = "max height of output images"
    )]
    max_height: u32,

    // 分割画像の最小の高さ
    #[clap(long = "min-height", default_value = "1000", help = "min height of output images")]
    min_height: u32,

    // 空白判定する高さ
    #[clap(long = "blank-height", default_value = "30", help = "height to decide blank spaces")]
    blank_height: usize,

    // 空白判定する分散しきい値
    #[clap(
        long = "blank-var-thr",
        default_value = "100.0",
        help = "variance threshold to decide blank spaces"
    )]
    blank_var_thr: f32,

    // 空白判定する幅 (横幅に対する割合)
    #[clap(
        long = "blank-left",
        default_value = "0",
        value_parser = clap::value_parser!(u32).range(0..100),
        help = "left portion to decide blank spaces (0-100)"
    )]
    blank_left: u32,

    // 空白判定する幅 (横幅に対する割合)
    #[clap(
        long = "blank-right",
        default_value = "100",
        value_parser = clap::value_parser!(u32).range(0..100),
        help = "right portion to decide blank spaces (0-100)"
    )]
    blank_right: u32,

    #[clap(long = "file-ext", help = "output file types [default: same as input]")]
    file_ext: Option<FileExt>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("cannot find blank space")]
    NoBlankSpace {
        file: PathBuf,
        y_end: u32,
        y_start: u32,
        blank_height: usize,
    },
    #[error("blank-height must be smaller than its image height")]
    InvalidBlankHeight,
    #[error("blank-left must be smaller than blank-right")]
    InvalidBlankLeft,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    println!("{:?}", cli);

    if cli.blank_left >= cli.blank_right {
        return Err(Error::InvalidBlankLeft.into());
    }

    if !&cli.output_dir.exists() {
        std::fs::create_dir(&cli.output_dir)?;
    }
    for file in std::fs::read_dir(&cli.output_dir)? {
        let path = file?.path();
        if path.extension().map_or(false, |e| e == "png" || e == "jpg") {
            std::fs::remove_file(path)?;
        }
    }

    for (file_no, file) in cli.input_path.iter().enumerate() {
        let im = Reader::open(file)?.decode()?;

        let (_means, vars): (Vec<_>, Vec<_>) = get_mean_var(
            &im,
            Some((cli.blank_left as f32 / 100.0, cli.blank_right as f32 / 100.0)),
        )
        .into_iter()
        .unzip();
        let vars = rolling(&vars, cli.blank_height).map_or(Err(Error::InvalidBlankHeight), Ok)?;
        let mut im_no = 0;
        let mut y_start = 0;
        let file_ext = match cli.file_ext {
            Some(ext) => match ext {
                FileExt::Jpg => "jpg",
                FileExt::Png => "png",
            },
            None => file.extension().and_then(|e| e.to_str()).unwrap_or("bin"),
        };

        while y_start < im.height() {
            let mut y_end = u32::min(y_start + cli.max_height, im.height());
            if y_end < im.height() {
                loop {
                    if vars[y_end as usize] < cli.blank_var_thr {
                        break;
                    }
                    if y_end - y_start < cli.min_height {
                        return Err(Error::NoBlankSpace {
                            file: file.clone(),
                            y_end,
                            y_start,
                            blank_height: cli.blank_height,
                        }
                        .into());
                    }
                    y_end -= 1;
                }
            }
            let filename = cli.output_dir.join(format!("{:02}-{:02}.{}", file_no, im_no, file_ext));
            im.crop_imm(0, y_start, im.width(), y_end - y_start + cli.margin)
                .save(&filename)?;
            println!("{:?}: {}-{}", filename, y_start, y_end);
            im_no += 1;
            y_start = y_end;
        }
    }

    Ok(())
}

fn rolling(v: &Vec<f32>, n: usize) -> Option<Vec<f32>> {
    if n > 0 && v.len() > n {
        Some(
            (0..v.len() - n + 1)
                .map(|i| v[i..i + n].iter().sum::<f32>() / n as f32)
                .collect(),
        )
    } else {
        None
    }
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
            let pixels: Vec<_> = row.skip(range.0).take(range.1 - range.0).collect();

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

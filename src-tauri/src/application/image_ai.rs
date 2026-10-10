use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use image::{
    imageops::{self, FilterType},
    DynamicImage, GrayImage, ImageReader, Luma, RgbImage, RgbaImage,
};
use tract_onnx::prelude::*;

use crate::{
    application::ffmpeg,
    domain::job::{ExecutionOutcome, ImageAiOperation},
};

pub const SUPPORTED_INPUTS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"];
pub const BACKGROUND_MODEL: &str = "u2netp.onnx";
pub const UPSCALE_MODEL: &str = "realesr-general-x4v3.onnx";

const MAX_BACKGROUND_PIXELS: u64 = 40_000_000;
const MAX_UPSCALE_PIXELS: u64 = 4_000_000;
const MAX_UPSCALE_SIDE: u64 = 16_384;
const SEGMENTATION_SIZE: usize = 320;
const TILE: usize = 128;
const TILE_PADDING: usize = 10;
const MODEL_SCALE: usize = 4;

type Model = Arc<TypedRunnableModel>;

pub struct ImageAiRequest<'a> {
    pub models_directory: &'a Path,
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub operation: ImageAiOperation,
    pub job_id: &'a str,
}

pub fn validate_request(
    input_path: &Path,
    output_path: &Path,
    operation: ImageAiOperation,
) -> Result<(), String> {
    if !input_path.is_absolute() || !output_path.is_absolute() {
        return Err("Input and output paths must be absolute".into());
    }
    if !input_path.is_file() {
        return Err("The selected input file does not exist".into());
    }
    let input_extension = extension(input_path).ok_or("The input file has no extension")?;
    if !SUPPORTED_INPUTS.contains(&input_extension.as_str()) {
        return Err(format!("Unsupported image format: {input_extension}"));
    }
    let allowed: &[&str] = match operation {
        ImageAiOperation::RemoveBackground => &["png"],
        ImageAiOperation::Upscale { .. } => &["png", "jpg", "jpeg"],
    };
    let output_extension = extension(output_path).unwrap_or_default();
    if !allowed.contains(&output_extension.as_str()) {
        return Err(format!(
            "The output file extension must be .{}",
            allowed.join(" or .")
        ));
    }
    if output_path.exists() {
        return Err("The output file already exists".into());
    }
    let output_parent = output_path
        .parent()
        .ok_or("The output location has no parent directory")?;
    if !output_parent.is_dir() {
        return Err("The output directory does not exist".into());
    }
    if input_path == output_path {
        return Err("Input and output paths must be different".into());
    }

    let (width, height) = ImageReader::open(input_path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|error| format!("The image could not be read: {error}"))?
        .into_dimensions()
        .map_err(|error| format!("The image could not be read: {error}"))?;
    let (width, height) = (u64::from(width), u64::from(height));
    match operation {
        ImageAiOperation::RemoveBackground => {
            if width * height > MAX_BACKGROUND_PIXELS {
                return Err("The image is too large (limit 40 megapixels)".into());
            }
        }
        ImageAiOperation::Upscale { scale } => {
            if ![2, 4].contains(&scale) {
                return Err("The upscaling factor must be 2 or 4".into());
            }
            if width * height > MAX_UPSCALE_PIXELS {
                return Err("The image is too large to upscale (limit 4 megapixels)".into());
            }
            if width.max(height) * u64::from(scale) > MAX_UPSCALE_SIDE {
                return Err("The upscaled image would be too large".into());
            }
        }
    }
    Ok(())
}

pub fn convert<F>(
    request: ImageAiRequest<'_>,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(request.input_path, request.output_path, request.operation)?;
    let staged_path = ffmpeg::staged_output_path(request.output_path, request.job_id)?;
    let _ = fs::remove_file(&staged_path);

    let source = ImageReader::open(request.input_path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|error| format!("The image could not be read: {error}"))?
        .decode()
        .map_err(|error| format!("The image could not be read: {error}"))?;
    let result = match request.operation {
        ImageAiOperation::RemoveBackground => {
            let model = load_model(
                &request.models_directory.join(BACKGROUND_MODEL),
                SEGMENTATION_SIZE,
            )?;
            report_progress(10);
            if cancelled.load(Ordering::Relaxed) {
                return Ok(ExecutionOutcome::Cancelled);
            }
            let cutout = remove_background(&model, &source)?;
            report_progress(90);
            Some(DynamicImage::ImageRgba8(cutout))
        }
        ImageAiOperation::Upscale { scale } => {
            let model = load_model(
                &request.models_directory.join(UPSCALE_MODEL),
                TILE + 2 * TILE_PADDING,
            )?;
            upscale(&model, &source, scale, &cancelled, &mut report_progress)?
        }
    };
    let Some(result) = result else {
        return Ok(ExecutionOutcome::Cancelled);
    };

    let saved = match extension(request.output_path).as_deref() {
        Some("png") => result.save_with_format(&staged_path, image::ImageFormat::Png),
        _ => DynamicImage::ImageRgb8(result.into_rgb8())
            .save_with_format(&staged_path, image::ImageFormat::Jpeg),
    };
    if let Err(error) = saved {
        let _ = fs::remove_file(&staged_path);
        return Err(format!("The result could not be saved: {error}"));
    }
    if fs::metadata(&staged_path).map_or(true, |metadata| metadata.len() == 0) {
        let _ = fs::remove_file(&staged_path);
        return Err("No output was produced".into());
    }
    fs::rename(&staged_path, request.output_path)
        .map_err(|error| format!("The result could not be finalized: {error}"))?;
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn load_model(path: &Path, size: usize) -> Result<Model, String> {
    let describe = |error: TractError| format!("The image model could not be loaded: {error}");
    tract_onnx::onnx()
        .model_for_path(path)
        .map_err(describe)?
        .with_input_fact(0, f32::fact([1, 3, size, size]).into())
        .map_err(describe)?
        .into_optimized()
        .map_err(describe)?
        .into_runnable()
        .map_err(describe)
}

fn run(model: &Model, input: tract_ndarray::Array4<f32>) -> Result<Tensor, String> {
    let outputs = model
        .run(tvec!(Tensor::from(input).into()))
        .map_err(|error| format!("The image model failed: {error}"))?;
    Ok(outputs[0].clone().into_tensor())
}

fn remove_background(model: &Model, source: &DynamicImage) -> Result<RgbaImage, String> {
    let rgb = source.to_rgb8();
    let small = imageops::resize(
        &rgb,
        SEGMENTATION_SIZE as u32,
        SEGMENTATION_SIZE as u32,
        FilterType::Lanczos3,
    );
    let peak = small.as_raw().iter().copied().max().unwrap_or(255).max(1) as f32;
    const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
    const DEVIATION: [f32; 3] = [0.229, 0.224, 0.225];
    let input = tract_ndarray::Array4::from_shape_fn(
        (1, 3, SEGMENTATION_SIZE, SEGMENTATION_SIZE),
        |(_, channel, y, x)| {
            let value = f32::from(small.get_pixel(x as u32, y as u32)[channel]) / peak;
            (value - MEAN[channel]) / DEVIATION[channel]
        },
    );
    let output = run(model, input)?;
    let values: Vec<f32> = output
        .to_plain_array_view::<f32>()
        .map_err(|error| format!("The image model returned an unexpected result: {error}"))?
        .iter()
        .copied()
        .collect();
    if values.len() != SEGMENTATION_SIZE * SEGMENTATION_SIZE {
        return Err("The image model returned an unexpected result".into());
    }
    let (low, high) = values
        .iter()
        .fold((f32::MAX, f32::MIN), |(low, high), value| {
            (low.min(*value), high.max(*value))
        });
    let range = (high - low).max(f32::EPSILON);
    let mask = GrayImage::from_fn(
        SEGMENTATION_SIZE as u32,
        SEGMENTATION_SIZE as u32,
        |x, y| {
            let value = (values[y as usize * SEGMENTATION_SIZE + x as usize] - low) / range;
            Luma([(value * 255.0).round() as u8])
        },
    );
    let mask = imageops::resize(&mask, rgb.width(), rgb.height(), FilterType::Lanczos3);

    let mut cutout = source.to_rgba8();
    for (pixel, mask) in cutout.pixels_mut().zip(mask.pixels()) {
        pixel[3] = (u16::from(pixel[3]) * u16::from(mask[0]) / 255) as u8;
    }
    Ok(cutout)
}

/// Upscales tile by tile; returns `None` when cancelled.
fn upscale(
    model: &Model,
    source: &DynamicImage,
    scale: u8,
    cancelled: &AtomicBool,
    report_progress: &mut impl FnMut(u8),
) -> Result<Option<DynamicImage>, String> {
    let rgb = source.to_rgb8();
    let (width, height) = (rgb.width() as usize, rgb.height() as usize);
    let (out_width, out_height) = (width * MODEL_SCALE, height * MODEL_SCALE);
    let mut canvas = RgbImage::new(out_width as u32, out_height as u32);
    let columns = width.div_ceil(TILE);
    let tiles = columns * height.div_ceil(TILE);
    let window = TILE + 2 * TILE_PADDING;

    for tile in 0..tiles {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let (left, top) = ((tile % columns) * TILE, (tile / columns) * TILE);
        let input = tract_ndarray::Array4::from_shape_fn((1, 3, window, window), |(_, c, y, x)| {
            // Replicating edge pixels keeps tiles at the model's fixed size.
            let source_x = (left + x).saturating_sub(TILE_PADDING).min(width - 1);
            let source_y = (top + y).saturating_sub(TILE_PADDING).min(height - 1);
            f32::from(rgb.get_pixel(source_x as u32, source_y as u32)[c]) / 255.0
        });
        let output = run(model, input)?;
        let view = output
            .to_plain_array_view::<f32>()
            .map_err(|error| format!("The image model returned an unexpected result: {error}"))?;
        if view.shape() != [1, 3, window * MODEL_SCALE, window * MODEL_SCALE] {
            return Err("The image model returned an unexpected result".into());
        }
        let tile_width = TILE.min(width - left);
        let tile_height = TILE.min(height - top);
        for y in 0..tile_height * MODEL_SCALE {
            for x in 0..tile_width * MODEL_SCALE {
                let mut pixel = [0u8; 3];
                for (c, value) in pixel.iter_mut().enumerate() {
                    let sample = view[[
                        0,
                        c,
                        TILE_PADDING * MODEL_SCALE + y,
                        TILE_PADDING * MODEL_SCALE + x,
                    ]];
                    *value = (sample.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                canvas.put_pixel(
                    (left * MODEL_SCALE + x) as u32,
                    (top * MODEL_SCALE + y) as u32,
                    image::Rgb(pixel),
                );
            }
        }
        report_progress((((tile + 1) * 95) / tiles) as u8);
    }

    let (final_width, final_height) = (
        (width * usize::from(scale)) as u32,
        (height * usize::from(scale)) as u32,
    );
    let rgb = if usize::from(scale) == MODEL_SCALE {
        canvas
    } else {
        imageops::resize(&canvas, final_width, final_height, FilterType::Lanczos3)
    };
    if !source.color().has_alpha() {
        return Ok(Some(DynamicImage::ImageRgb8(rgb)));
    }
    let alpha = imageops::resize(
        &source.to_rgba8(),
        final_width,
        final_height,
        FilterType::Lanczos3,
    );
    let combined = RgbaImage::from_fn(final_width, final_height, |x, y| {
        let color = rgb.get_pixel(x, y);
        image::Rgba([color[0], color[1], color[2], alpha.get_pixel(x, y)[3]])
    });
    Ok(Some(DynamicImage::ImageRgba8(combined)))
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn write_png(path: &Path, width: u32, height: u32) {
        RgbImage::from_pixel(width, height, image::Rgb([200, 30, 30]))
            .save(path)
            .unwrap();
    }

    #[test]
    fn validation_rejects_bad_requests() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("photo.png");
        write_png(&input, 40, 30);
        let upscale = ImageAiOperation::Upscale { scale: 2 };

        assert!(validate_request(
            &input,
            &directory.path().join("out.jpg"),
            ImageAiOperation::RemoveBackground
        )
        .unwrap_err()
        .contains("must be .png"));
        assert!(
            validate_request(&input, &directory.path().join("out.webp"), upscale)
                .unwrap_err()
                .contains("must be .png or .jpg")
        );
        assert!(validate_request(
            &input,
            &directory.path().join("out.png"),
            ImageAiOperation::Upscale { scale: 3 }
        )
        .unwrap_err()
        .contains("2 or 4"));
        assert!(validate_request(&input, &input, upscale)
            .unwrap_err()
            .contains("already exists"));
        assert!(validate_request(&input, &directory.path().join("out.png"), upscale).is_ok());

        let large = directory.path().join("large.png");
        write_png(&large, 2100, 2100);
        assert!(
            validate_request(&large, &directory.path().join("big.png"), upscale)
                .unwrap_err()
                .contains("too large")
        );
        assert!(validate_request(
            &large,
            &directory.path().join("cutout.png"),
            ImageAiOperation::RemoveBackground
        )
        .is_ok());
    }

    fn models() -> PathBuf {
        platform::find_image_ai_models(None).expect("run `pnpm fetch:engines` first")
    }

    fn run_operation(
        input: &Path,
        output: &Path,
        operation: ImageAiOperation,
        cancelled: bool,
    ) -> Result<ExecutionOutcome, String> {
        convert(
            ImageAiRequest {
                models_directory: &models(),
                input_path: input,
                output_path: output,
                operation,
                job_id: "1",
            },
            Arc::new(AtomicBool::new(cancelled)),
            |_| {},
        )
    }

    #[test]
    #[ignore = "needs the fetched image models"]
    fn removes_the_background_around_a_subject() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("subject.png");
        // A bright red disc on a flat grey background.
        RgbImage::from_fn(320, 320, |x, y| {
            let (dx, dy) = (x as f32 - 160.0, y as f32 - 160.0);
            if dx * dx + dy * dy < 90.0 * 90.0 {
                image::Rgb([220, 20, 20])
            } else {
                image::Rgb([120, 120, 120])
            }
        })
        .save(&input)
        .unwrap();
        let output = directory.path().join("cutout.png");

        run_operation(&input, &output, ImageAiOperation::RemoveBackground, false).unwrap();

        let cutout = image::open(&output).unwrap().to_rgba8();
        assert_eq!(cutout.dimensions(), (320, 320));
        let (centre, corner) = (cutout.get_pixel(160, 160)[3], cutout.get_pixel(4, 4)[3]);
        assert!(centre > 200, "subject alpha {centre}");
        assert!(corner < 60, "background alpha {corner}");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    #[ignore = "needs the fetched image models"]
    fn upscales_two_and_four_times_without_seams() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("gradient.png");
        RgbImage::from_fn(150, 90, |x, y| {
            image::Rgb([(x * 255 / 150) as u8, (y * 255 / 90) as u8, 128])
        })
        .save(&input)
        .unwrap();

        for (scale, name) in [(2u8, "two.png"), (4u8, "four.png")] {
            let output = directory.path().join(name);
            run_operation(&input, &output, ImageAiOperation::Upscale { scale }, false).unwrap();
            let result = image::open(&output).unwrap().to_rgb8();
            assert_eq!(
                result.dimensions(),
                (150 * u32::from(scale), 90 * u32::from(scale))
            );
            // Smooth input stays smooth: no tile seam jumps across the tile boundary.
            let seam = u32::from(scale) * 128;
            for y in [10, 40, 80] {
                let (a, b) = (
                    result.get_pixel(seam - 1, y * u32::from(scale)),
                    result.get_pixel(seam, y * u32::from(scale)),
                );
                for c in 0..3 {
                    assert!(
                        a[c].abs_diff(b[c]) < 12,
                        "seam at scale {scale}: {a:?} vs {b:?}"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "needs the fetched image models"]
    fn cancellation_leaves_no_files_behind() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("photo.png");
        write_png(&input, 200, 200);
        let output = directory.path().join("out.png");

        let outcome = run_operation(
            &input,
            &output,
            ImageAiOperation::Upscale { scale: 4 },
            true,
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Cancelled));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

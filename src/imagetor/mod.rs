use core::fmt;
use std::{error::Error, vec};

use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Rgba};

pub mod utils;

const CHANNELS: usize = 4;

pub fn to_tensor(image: DynamicImage) -> Vec<Vec<Vec<f32>>> {
    let (width, height) = image.dimensions();
    let mut tensor: Vec<Vec<Vec<f32>>> =
        vec![vec![vec![0f32; CHANNELS]; width as usize]; height as usize];

    tensor.iter_mut().enumerate().for_each(|(y, row)| {
        for (x, pixel) in row.iter_mut().enumerate() {
            *pixel = image
                .get_pixel(x as u32, y as u32)
                .0
                .into_iter()
                .map(|c| c as f32 / 255.0)
                .collect();
        }
    });
    tensor
}

#[allow(dead_code)]
pub fn to_image_buffer(tensor: Vec<Vec<Vec<f32>>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = (tensor[0].len(), tensor.len());
    let mut image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::new(width as u32, height as u32);

    image_buffer.enumerate_pixels_mut().for_each(|pixel| {
        let x = pixel.0 as usize;
        let y = pixel.1 as usize;

        let r = (tensor[y][x][0] * 255.0) as u8;
        let g = (tensor[y][x][1] * 255.0) as u8;
        let b = (tensor[y][x][2] * 255.0) as u8;
        let a = (tensor[y][x][3] * 255.0) as u8;

        *pixel.2 = Rgba([r, g, b, a]);
    });
    image_buffer
}

#[allow(dead_code)]
pub fn to_image(tensor: Vec<Vec<Vec<f32>>>) -> DynamicImage {
    let image_buffer = to_image_buffer(tensor);
    DynamicImage::ImageRgba8(image_buffer)
}

#[allow(dead_code)]
pub fn resize(tensor: &mut Vec<Vec<Vec<f32>>>, width: usize, height: usize) {
    let (old_width, old_height) = (tensor[0].len(), tensor.len());
    let mut temp_tensor = vec![vec![vec![0f32; CHANNELS]; width]; height];

    for y in 0..height {
        for x in 0..width {
            let old_x = (x as f32 * old_width as f32) / width as f32;
            let old_y = (y as f32 * old_height as f32) / height as f32;

            let x0 = old_x as i32;
            let y0 = old_y as i32;
            let dx = old_x - x0 as f32;
            let dy = old_y - y0 as f32;

            if x0 < 0 || x0 >= (old_width - 1) as i32 || y0 < 0 || y0 >= (old_height - 1) as i32 {
                continue;
            }

            for c in 0..CHANNELS {
                temp_tensor[y][x][c] =
                    (1.0 - dx) * (1.0 - dy) * (*tensor)[y0 as usize][x0 as usize][c]
                        + dx * (1.0 - dy) * tensor[y0 as usize][(x0 + 1) as usize][c]
                        + (1.0 - dx) * dy * tensor[(y0 + 1) as usize][x0 as usize][c]
                        + dx * dy * tensor[(y0 + 1) as usize][(x0 + 1) as usize][c];
            }
        }
    }
    *tensor = temp_tensor;
}

pub fn fit_center(image1: &DynamicImage, image2: &DynamicImage) -> DynamicImage {
    let (width1, height1) = image1.dimensions();
    let (width2, height2) = image2.dimensions();

    let factor = mean_center(width1, height1, width2, height2);
    let nwidth = (width1 as f32 * factor) as u32;
    let nheight = (height1 as f32 * factor) as u32;

    image1.resize(nwidth, nheight, imageops::FilterType::Lanczos3)
}

fn mean_center(tx: u32, ty: u32, dx: u32, dy: u32) -> f32 {
    let mut factor = 1f32;

    if tx > dx || ty > dy {
        let scale_x = dx as f32 / tx as f32;
        let scale_y = dy as f32 / ty as f32;
        factor = scale_x.min(scale_y);
    }
    factor
}

#[derive(Debug)]
struct ArrayEmptyError;

impl fmt::Display for ArrayEmptyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Array is empty")
    }
}

impl Error for ArrayEmptyError {}

pub fn addwatermark(
    logoimage: &Vec<Vec<Vec<f32>>>,
    basedimage: &mut Vec<Vec<Vec<f32>>>,
) -> Result<(), Box<dyn Error>> {
    if logoimage.len() == 0 || basedimage.len() == 0 {
        return Err(Box::new(ArrayEmptyError));
    }

    let bwidth = basedimage[0].len();
    let bheight = basedimage.len();
    let lwidth = logoimage[0].len();
    let lheight = logoimage.len();

    let offset_x = (bwidth - lwidth) / 2;
    let offset_y = (bheight - lheight) / 2;

    basedimage
        .iter_mut()
        .skip(offset_y)
        .take(lheight)
        .enumerate()
        .for_each(|(y, row)| {
            for x in offset_x..offset_x + lwidth {
                for c in 0..CHANNELS - 1 {
                    let alpha = logoimage[y][x - offset_x][3];
                    row[x][c] = (1.0 - alpha) * row[x][c] + alpha * logoimage[y][x - offset_x][c];
                }
                row[x][3] = 1.0;
            }
        });
    Ok(())
}

#[allow(dead_code)]
pub fn flip_vertical(tensor: &mut Vec<Vec<Vec<f32>>>) {
    let (width, height) = (tensor[0].len(), tensor.len());

    for y in 0..height / 2 {
        let top_row = tensor[y].clone();
        let bottom_row = tensor[height - y - 1].clone();

        for x in 0..width {
            tensor[y][x] = bottom_row[x].clone();
            tensor[height - y - 1][x] = top_row[x].clone();
        }
    }
}

#[allow(dead_code)]
pub fn flip_horizontal(tensor: &mut Vec<Vec<Vec<f32>>>) {
    let (width, height) = (tensor[0].len(), tensor.len());

    for y in 0..height {
        for x in 0..width / 2 {
            let left_col = tensor[y][x].clone();
            let right_col = tensor[y][width - x - 1].clone();

            tensor[y][x] = right_col;
            tensor[y][width - x - 1] = left_col;
        }
    }
}

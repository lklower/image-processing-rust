use core::fmt;
use image::{codecs::jpeg::JpegEncoder, DynamicImage, GenericImageView};
use printpdf::{ColorBits, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px};
use std::{
    fs::{self, File},
    io::{self, BufWriter},
    path::{Path, PathBuf},
};

use crate::imagetor::to_image_buffer;

#[derive(Debug)]
pub enum ImageFinderError {
    IOError(std::io::Error),
}

impl From<std::io::Error> for ImageFinderError {
    fn from(value: std::io::Error) -> Self {
        ImageFinderError::IOError(value)
    }
}

impl fmt::Display for ImageFinderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageFinderError::IOError(ref e) => write!(f, "IOError: {}", e),
        }
    }
}

impl std::error::Error for ImageFinderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ImageFinderError::IOError(e) => Some(e),
        }
    }
}

pub struct ImageFinder {
    paths: PathBuf,
    paths_buff: Vec<PathBuf>,
}

impl ImageFinder {
    pub fn new(paths: PathBuf) -> Self {
        Self {
            paths,
            paths_buff: Vec::new(),
        }
    }

    fn image_path_finding(&mut self) -> Result<(), ImageFinderError> {
        for entry in fs::read_dir(self.paths.clone())? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                self.paths_buff.push(path);
            }
        }
        Ok(())
    }

    pub fn get_paths(&mut self) -> Result<Vec<&Path>, ImageFinderError> {
        let _ = self.image_path_finding()?;
        Ok(self.paths_buff.iter().map(|p| p.as_path()).collect())
    }
}

pub struct Utils;

#[derive(Debug)]
pub enum UtilsError {
    ImageError(image::ImageError),
    IOError(std::io::Error),
    PrintError(printpdf::errors::Error),
}

impl fmt::Display for UtilsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UtilsError::ImageError(ref e) => write!(f, "IOError: {}", e),
            UtilsError::IOError(ref e) => write!(f, "IOError: {}", e),
            UtilsError::PrintError(ref e) => write!(f, "PrintError: {}", e),
        }
    }
}

impl std::error::Error for UtilsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UtilsError::ImageError(e) => Some(e),
            UtilsError::IOError(e) => Some(e),
            UtilsError::PrintError(e) => Some(e),
        }
    }
}

impl From<image::ImageError> for UtilsError {
    fn from(e: image::ImageError) -> Self {
        UtilsError::ImageError(e)
    }
}

impl From<std::io::Error> for UtilsError {
    fn from(e: std::io::Error) -> Self {
        UtilsError::IOError(e)
    }
}

impl From<printpdf::errors::Error> for UtilsError {
    fn from(e: printpdf::errors::Error) -> Self {
        UtilsError::PrintError(e)
    }
}

impl Utils {
    pub fn open_image(self, path: &Path) -> Result<DynamicImage, image::ImageError> {
        let mut a = image::io::Reader::open(path)?;
        image::io::Reader::no_limits(&mut a);
        a.decode()
    }

    pub fn save_image(self, tensor: Vec<Vec<Vec<f32>>>, filename: &str) -> Result<(), UtilsError> {
        let image_buffer = to_image_buffer(tensor);
        let file = File::create(filename).unwrap();
        let mut writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(&mut writer, 100);

        if let Err(e) = encoder.encode_image(&image_buffer) {
            println!("Failed to save image: {}", e);
            return Err(UtilsError::ImageError(e));
        }
        println!("Image saved successfully!");
        Ok(())
    }

    fn to_px(&self, value: f32) -> u32 {
        return (value / 25.4 * 300.0) as u32;
    }

    pub fn generate_pdf(&self, filename: &String) -> Result<(), UtilsError> {
        let (pdf_width, pdf_height) = (Mm(210.0), Mm(297.0));
        let (pdf_width_pixel, pdf_height_pixel) =
            (self.to_px(pdf_width.0), self.to_px(pdf_height.0));

        let (doc, page, layer) =
            PdfDocument::new(filename, pdf_width, pdf_height, "Vera Smith Design");
        let current_layer = doc.get_page(page).get_layer(layer);

        let image_file = File::open(filename)?;
        let reader = io::BufReader::new(image_file);

        let image_reader = image::io::Reader::new(reader).with_guessed_format();
        let reader = image_reader?;
        let img = reader.decode()?;
        let (w, h) = img.dimensions();

        let img = img.to_rgb8();
        let mut image_bytes = Vec::new();
        image_bytes.extend_from_slice(img.as_raw());

        let image = ImageXObject {
            width: Px(w as usize),
            height: Px(h as usize),
            color_space: printpdf::ColorSpace::Rgb,
            bits_per_component: ColorBits::Bit8,
            interpolate: true,
            image_data: image_bytes,
            image_filter: None,
            smask: None,
            clipping_bbox: None,
        };

        let mut aspect_ratio = 1.0;
        if w > pdf_width_pixel || h > pdf_height_pixel {
            let ratio_x = pdf_width_pixel as f32 / w as f32;
            let ratio_y = pdf_height_pixel as f32 / h as f32;
            aspect_ratio = ratio_x.min(ratio_y);
        }

        let new_image_width = (aspect_ratio * w as f32) as u32;
        let new_image_height = (aspect_ratio * h as f32) as u32;
        let translate_x = (pdf_width_pixel - new_image_width) / 2;
        let translate_x = translate_x as f32 / 300.0 * 25.4;
        let translate_y = (pdf_height_pixel - new_image_height) / 2;
        let translate_y = translate_y as f32 / 300.0 * 25.4;

        Image::from(image).add_to_layer(
            current_layer,
            ImageTransform {
                translate_x: Some(Mm(translate_x)),
                translate_y: Some(Mm(translate_y)),
                rotate: None,
                scale_x: Some(aspect_ratio),
                scale_y: Some(aspect_ratio),
                dpi: Some(300.0),
            },
        );

        let pdffile = File::create(filename.replace(".jpg", ".pdf"))?;
        let mut writer = BufWriter::new(pdffile);

        if let Err(e) = doc.save(&mut writer) {
            return Err(UtilsError::PrintError(e));
        }
        Ok(())
    }
}

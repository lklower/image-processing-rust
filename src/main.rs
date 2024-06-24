use image::{self, GenericImageView};
use std::{
    env::current_dir,
    io::{stdin, stdout, Read, Write},
    path::Path,
    time::{Duration, Instant},
};

mod imagetor;
use imagetor::{addwatermark, fit_center, to_tensor, utils::ImageFinder};
use imagetor::{to_image_buffer, utils::Utils};

fn main() {
    let start_time: Instant = Instant::now();

    let current_binding = current_dir().unwrap();
    let current_path = Path::new(&current_binding);
    let images_path = current_path.join("images");

    let mut finder = ImageFinder::new(images_path.clone());
    let images: Vec<&Path> = finder.get_paths().unwrap();

    println!("Start to opening image ...");

    // Opening logo
    let logo_binding = current_path.join("logo.png");
    let logo_path = Path::new(&logo_binding);
    let logo = Utils.open_image(logo_path).unwrap();

    println!("Logo Original size: {:?}", logo.dimensions());

    for (i, image) in images.iter().enumerate() {
        // Generating images path
        let default_filename = format!("_{}.jpg", i);
        let filename = image
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&default_filename);
        let image_binding = current_path.join(images_path.join(image));
        let image_path = Path::new(&image_binding);

        // Opening image
        let image = Utils.open_image(image_path).unwrap();

        // Clone the logo to avoid modifying the original logo
        let mut logo = logo.clone();
        logo = fit_center(&logo, &image);

        println!(
            "Start to converting image to tensor ...resized: {:?}",
            logo.dimensions()
        );

        // converting image and logo to 3D Tensor
        let mut tensor1 = to_tensor(image);
        let tensor2 = to_tensor(logo);

        println!("Start to adding watermark ...");

        if let Err(e) = addwatermark(&tensor2, &mut tensor1) {
            println!("Failed to add watermark: {}", e);
            return;
        }
        println!("watermark added successfully!");

        // use imagetor::{flip_horizontal, flip_vertical};
        // flip_vertical(&mut tensor1);
        // flip_horizontal(&mut tensor1);

        println!("Start to saving image ...");

        let new_filename = &format!("output-{}", filename).replace(" ", "-");
        if let Ok(_ok) = Utils.save_image(tensor1.clone(), new_filename) {
            println!("{} saved successfully!", new_filename);
            println!("Start to converting image to PDF ...");
            let new_filename = new_filename.replace(".jpg", ".pdf");
            if let Ok(()) = Utils.generate_pdf(&new_filename, to_image_buffer(tensor1)) {
                println!("{} created successfully!", new_filename);
            }
        } else {
            println!("Failed to save image: {}", new_filename);
        }
    }

    println!("All images processed successfully!");

    let elapsed: Duration = start_time.elapsed();
    println!("Elapsed time: {:?}", elapsed);

    let mut stdout = stdout();
    //stdout.write(b"\x1b[?25h").unwrap();
    stdout.write(b"Press [enter] key to exit...\n").unwrap();
    stdout.lock().flush().unwrap();

    let _ = stdin().read_exact(&mut [0]).unwrap();
}

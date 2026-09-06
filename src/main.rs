use std::io::{Error, Write};
use thumbcache::{get_bmp_with, ThumbSize, SIIGBF_THUMBNAILONLY};

pub fn main() -> Result<(), Error> {
  let bmp = match get_bmp_with(r"C:\path-to-file.jpeg", ThumbSize::S96, SIIGBF_THUMBNAILONLY) {
    Ok(bytes) => bytes,
    Err(error) => {
      println!("Error: {}", error);

      return Ok(());
    }
  };

  let mut file_out = std::fs::File::create("./out.bmp")?;
  file_out.write_all(&bmp)?;

  Ok(())
}
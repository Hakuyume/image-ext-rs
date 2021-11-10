pub use image;
use image::{DynamicImage, ImageFormat};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Exif(#[from] exif::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("unknown exif orientation tag `{0}`")]
    UnknownExifOrientationTag(u32),
}

pub fn load<R: BufRead + Seek>(mut r: R, format: ImageFormat) -> Result<DynamicImage, Error> {
    let image = image::load(&mut r, format)?;

    match format {
        ImageFormat::Avif
        | ImageFormat::Jpeg
        | ImageFormat::Png
        | ImageFormat::Tiff
        | ImageFormat::WebP => {
            r.seek(SeekFrom::Start(0))?;
            let exif = match exif::Reader::new().read_from_container(&mut r) {
                Ok(exif) => Ok(Some(exif)),
                Err(exif::Error::NotFound(_)) => Ok(None),
                Err(e) => Err(e),
            }?;

            match exif
                .as_ref()
                .and_then(|exif| exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY))
                .and_then(|field| field.value.get_uint(0))
            {
                // https://magnushoff.com/articles/jpeg-orientation/
                Some(1) => Ok(image),
                Some(2) => Ok(image.fliph()),
                Some(3) => Ok(image.rotate180()),
                Some(4) => Ok(image.flipv()),
                Some(5) => Ok(image.fliph().rotate270()),
                Some(6) => Ok(image.rotate90()),
                Some(7) => Ok(image.fliph().rotate90()),
                Some(8) => Ok(image.rotate270()),
                Some(tag) => Err(Error::UnknownExifOrientationTag(tag)),
                None => Ok(image),
            }
        }
        _ => Ok(image),
    }
}

pub fn load_from_memory(buffer: &[u8]) -> Result<DynamicImage, Error> {
    load_from_memory_with_format(buffer, image::guess_format(buffer)?)
}

pub fn load_from_memory_with_format(
    buf: &[u8],
    format: ImageFormat,
) -> Result<DynamicImage, Error> {
    load(Cursor::new(buf), format)
}

pub fn open<P>(path: P) -> Result<DynamicImage, Error>
where
    P: AsRef<Path>,
{
    load(
        BufReader::new(File::open(path.as_ref())?),
        ImageFormat::from_path(path.as_ref())?,
    )
}

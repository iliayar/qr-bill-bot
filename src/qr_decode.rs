use core::fmt;

use log::{info, error};
use rqrr;
use image::{self, ImageError};

#[derive(Debug)]
pub enum QrDecodeError {
    IOError(String),
    NotFound,
    DecodeFailed,
}

impl fmt::Display for QrDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	match self {
	    Self::IOError(err) => write!(f, "Could not load qr from file: {}", err),
	    Self::NotFound => write!(f, "Could not detect qr on image"),
	    Self::DecodeFailed => write!(f, "Faild to decode any qr"),
	}
    }
}

impl std::error::Error for QrDecodeError {}

impl From<ImageError> for QrDecodeError {
    fn from(err: ImageError) -> Self {
	Self::IOError(err.to_string())
    }
}

pub fn decode_qr(path: impl AsRef<str>) -> Result<String, QrDecodeError> {
    let img = image::open(path.as_ref())?.to_luma8();
    let mut img = rqrr::PreparedImage::prepare(img);
    info!("Image read");

    let grids = img.detect_grids();

    if grids.len() == 0 {
	error!("Could not find grids");
	return Err(QrDecodeError::NotFound);
    }

    let mut content: Option<String> = None;
    for grid in grids {
	match grid.decode() {
	    Ok((_, cont)) => {
		info!("Successfully decode grid");
		content = Some(cont)
	    },
	    Err(_) => error!("Coulnd not decode grid")
	}
    }

    return match content {
	None => Err(QrDecodeError::DecodeFailed),
	Some(content) => Ok(content),
    };
}

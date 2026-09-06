mod com;

use crate::com::ComLibrary;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{Error, Interface, Owned, PCWSTR};
use windows::Win32::Foundation::{E_FAIL, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPFILEHEADER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
};
use windows::Win32::UI::Shell::{IShellItem, IShellItemImageFactory, SHCreateItemFromParsingName};

pub use windows::Win32::UI::Shell::{
    SIIGBF, SIIGBF_BIGGERSIZEOK, SIIGBF_CROPTOSQUARE, SIIGBF_ICONBACKGROUND, SIIGBF_ICONONLY,
    SIIGBF_INCACHEONLY, SIIGBF_MEMORYONLY, SIIGBF_RESIZETOFIT, SIIGBF_SCALEUP, SIIGBF_THUMBNAILONLY,
    SIIGBF_WIDETHUMBNAILS,
};

thread_local! {
    static COM_LIBRARY: ComLibrary = ComLibrary::init();
}

struct CompatibleDc(HDC);

impl Drop for CompatibleDc {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }
}

fn create_shell_item(file_path: &Path) -> Result<IShellItem, Error> {
    let wide_file_name: Vec<u16> = file_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe { SHCreateItemFromParsingName(PCWSTR(wide_file_name.as_ptr()), None) }
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbSize {
    /// 16x16 pixels
    S16,
    /// 32x32 pixels
    S32,
    /// 48x48 pixels
    S48,
    /// 96x96 pixels
    S96,
    /// 256x256 pixels
    S256,
    /// 768x768 pixels
    S768,
    /// 1280x1280 pixels
    S1280,
    /// 1920x1920 pixels
    S1920,
    /// 2560x2560 pixels
    S2560,
    /// Custom thumbnail size as (width, height) in pixels
    Custom(i32, i32),
}

impl ThumbSize {
    pub fn to_size(self) -> SIZE {
        match self {
            Self::S16 => SIZE { cx: 16, cy: 16 },
            Self::S32 => SIZE { cx: 32, cy: 32 },
            Self::S48 => SIZE { cx: 48, cy: 48 },
            Self::S96 => SIZE { cx: 96, cy: 96 },
            Self::S256 => SIZE { cx: 256, cy: 256 },
            Self::S768 => SIZE { cx: 768, cy: 768 },
            Self::S1280 => SIZE { cx: 1280, cy: 1280 },
            Self::S1920 => SIZE { cx: 1920, cy: 1920 },
            Self::S2560 => SIZE { cx: 2560, cy: 2560 },
            Self::Custom(w, h) => SIZE { cx: w, cy: h },
        }
    }
}

/// Returns thumbnail bitmap bits.
///
/// The thumbnail will be no larger than the specified width and height.
/// On first use per thread this initializes COM as a multithreaded apartment.
///
/// This fails if Windows has no thumbnail.
pub fn get_bmp(file_path: impl AsRef<Path>, size: ThumbSize) -> Result<Vec<u8>, Error> {
    get_bmp_with(file_path, size, SIIGBF_THUMBNAILONLY)
}

/// Returns thumbnail bitmap bits using the given [`IShellItemImageFactory::GetImage`] flags.
pub fn get_bmp_with(
    file_path: impl AsRef<Path>,
    size: ThumbSize,
    flags: SIIGBF,
) -> Result<Vec<u8>, Error> {
    COM_LIBRARY.with(|_| {});

    let hbitmap = {
        let shell_item = create_shell_item(file_path.as_ref())?;
        let factory: IShellItemImageFactory = shell_item.cast()?;

        unsafe { Owned::new(factory.GetImage(size.to_size(), flags)?) }
    };

    let mut bmp = BITMAP::default();

    if unsafe {
        GetObjectW(
            (*hbitmap).into(),
            size_of::<BITMAP>() as i32,
            Some((&raw mut bmp).cast()),
        )
    } == 0
    {
        return Err(Error::from_hresult(E_FAIL));
    }

    let hdc = unsafe { CreateCompatibleDC(None) };

    if hdc.is_invalid() {
        return Err(Error::from_hresult(E_FAIL));
    }

    let hdc = CompatibleDc(hdc);

    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bmp.bmWidth,
            biHeight: -bmp.bmHeight,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits = vec![0u8; 4 * bmp.bmWidth.unsigned_abs() as usize * bmp.bmHeight.unsigned_abs() as usize];

    if unsafe {
        GetDIBits(
            hdc.0,
            *hbitmap,
            0,
            bmp.bmHeight.unsigned_abs(),
            Some(bits.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    } == 0 {
        return Err(Error::from_hresult(E_FAIL));
    }

    let bitmap_header_size = size_of::<BITMAPFILEHEADER>() + size_of::<BITMAPINFOHEADER>();
    let bitmap_file_size = bitmap_header_size + bits.len();

    let file_header = BITMAPFILEHEADER {
        bfType: 0x4D42,
        bfSize: bitmap_file_size as u32,
        bfOffBits: bitmap_header_size as u32,
        ..Default::default()
    };

    let mut result = Vec::with_capacity(bitmap_file_size);

    result.extend_from_slice(unsafe {
        std::slice::from_raw_parts(
            (&raw const file_header).cast(),
            size_of::<BITMAPFILEHEADER>(),
        )
    });

    result.extend_from_slice(unsafe {
        std::slice::from_raw_parts(
            (&raw const bitmap_info.bmiHeader).cast(),
            size_of::<BITMAPINFOHEADER>(),
        )
    });

    result.extend_from_slice(&bits);

    Ok(result)
}

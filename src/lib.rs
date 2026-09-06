mod com;

use crate::com::ComLibrary;
use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPFILEHEADER,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::UI::Shell::{
    IShellItem, IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_THUMBNAILONLY,
};

thread_local! {
    static COM_LIBRARY: ComLibrary = ComLibrary::init();
}

fn create_shell_item(file_name: &str) -> Result<IShellItem, windows::core::Error> {
    let wide_file_name: Vec<u16> = OsStr::new(file_name).encode_wide().chain(Some(0)).collect();
    unsafe { SHCreateItemFromParsingName(PCWSTR(wide_file_name.as_ptr()), None) }
}

#[derive(Debug)]
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

/// Returns thumbnail bitmap bits
/// Thumbnail will be no larger than the specified width and height.
///
/// ```
/// let bmp = thumbcache::get_bmp(r"C:\path-to-file.jpeg", thumbcache::ThumbSize::S96).unwrap()
/// ```
pub fn get_bmp(file_path: &str, size: ThumbSize) -> Result<Vec<u8>, windows::core::Error> {
    COM_LIBRARY.with(|_| {});

    let hbitmap = unsafe {
        let shell_item = create_shell_item(file_path)?;
        let factory: IShellItemImageFactory = shell_item.cast()?;

        factory.GetImage(size.to_size(), SIIGBF_THUMBNAILONLY)?
    };

    let hgdiobj: HGDIOBJ = hbitmap.into();

    unsafe {
        let mut bmp = BITMAP::default();

        GetObjectW(
            hgdiobj,
            size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut _),
        );

        let hdc = CreateCompatibleDC(None);

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

        let byte_size = 4 * bmp.bmWidth.abs() * bmp.bmHeight.abs();
        let mut bits = vec![0u8; byte_size as usize];

        let get_di_bits_result = GetDIBits(
            hdc,
            hbitmap,
            0,
            bmp.bmHeight.unsigned_abs(),
            Some(bits.as_mut_ptr() as _),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        let _ = DeleteDC(hdc);
        let _ = DeleteObject(hgdiobj);

        if get_di_bits_result == 0 {
            return Err(windows::core::Error::from_thread());
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

        result.extend_from_slice(std::slice::from_raw_parts(
            &file_header as *const _ as *const u8,
            size_of::<BITMAPFILEHEADER>(),
        ));

        result.extend_from_slice(std::slice::from_raw_parts(
            &bitmap_info.bmiHeader as *const _ as *const u8,
            size_of::<BITMAPINFOHEADER>(),
        ));

        result.extend_from_slice(&bits);

        Ok(result)
    }
}

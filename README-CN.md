# thumbcache

利用Windows的thumbcache获取文件的BMP格式预览图。

## 使用方法

尝试获取非图片文件（如.zip或.exe）的预览会导致错误。

```rs
use std::io::{Error, Write};

pub fn main() -> Result<(), Error> {
  let bmp = thumbcache::get_bmp(r"C:\文件路径.jpeg", thumbcache::ThumbSize::S96)?;
  
  let mut file_out = std::fs::File::create("./out.bmp")?;
  let _ = file_out.write_all(&bmp);
  
  Ok(())
}
```

## 示例

### 将BMP转换为JPEG

Windows只能返回BMP格式，但有时这个格式可能不太方便。这个示例使用[image](https://github.com/image-rs/image) crate将BMP转换为JPEG。

```rs
use thumbcache::{get_bmp};
use image::{load_from_memory};

fn main() {
  let bmp = get_bmp(r"C:\文件路径.jpeg", thumbcache::ThumbSize::S256).unwrap();

  let image = load_from_memory(&bmp).unwrap();

  let mut buf: Vec<u8> = Vec::new();
  let mut writer = std::io::Cursor::new(&mut buf);

  image.write_to(&mut writer, image::ImageFormat::Jpeg).unwrap();

  std::fs::write("./output.jpeg", buf).unwrap();
}
```

## 参考来源

https://stackoverflow.com/questions/14207618/get-bytes-from-hbitmap

https://stackoverflow.com/questions/21751747/extract-thumbnail-for-any-file-in-windows

https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellitemimagefactory-getimage
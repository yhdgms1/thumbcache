use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED,
};

pub struct ComLibrary {
    initialized: bool,
}

impl ComLibrary {
    pub fn init() -> ComLibrary {
        unsafe {
            let hr = CoInitializeEx(
                None,
                COINIT(COINIT_MULTITHREADED.0 | COINIT_DISABLE_OLE1DDE.0),
            );

            Self {
                initialized: hr.is_ok(),
            }
        }
    }
}

impl Drop for ComLibrary {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

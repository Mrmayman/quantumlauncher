use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

pub struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    pub fn new() -> windows::core::Result<Self> {
        use windows::Win32::Foundation::RPC_E_CHANGED_MODE;

        unsafe {
            match CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() {
                Ok(()) => Ok(Self {
                    should_uninit: true,
                }),
                Err(e) if e.code() == RPC_E_CHANGED_MODE => {
                    // Already initialized with different model — OK
                    Ok(Self {
                        should_uninit: false,
                    })
                }
                Err(e) => Err(e),
            }
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe { CoUninitialize() };
        }
    }
}

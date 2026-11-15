use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// Work area (desktop minus taskbar/app bars) of a monitor, in physical pixels,
/// and its effective DPI scale (1.0 = 100 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub scale: f64,
}

impl WorkArea {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
}

/// The primary monitor. (Multi-monitor placement is a planned option.)
pub fn primary_work_area() -> WorkArea {
    unsafe {
        let hmon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let rc = if GetMonitorInfoW(hmon, &mut info).as_bool() {
            info.rcWork
        } else {
            RECT { left: 0, top: 0, right: 1920, bottom: 1040 }
        };
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_err() || dpi_x == 0 {
            dpi_x = 96;
        }
        WorkArea {
            left: rc.left,
            top: rc.top,
            right: rc.right,
            bottom: rc.bottom,
            scale: dpi_x as f64 / 96.0,
        }
    }
}

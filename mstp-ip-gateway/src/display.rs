//! LCD Display driver for M5StickC Plus2
//!
//! ST7789V2 LCD (135x240 pixels)
//! SPI pins: MOSI=15, SCK=13, CS=5, DC=14, RST=12, BL=27
//!
//! Supports multiple screens cycled with Button A:
//! - Screen 0: Status (traffic stats, loop time)
//! - Screen 1: Connection (WiFi, MSTP status, baud, address)
//! - Screen 2: Splash (BACman logo)

use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{ascii::{FONT_6X13, FONT_9X18_BOLD, FONT_10X20}, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{OutputPin, PinDriver},
    spi::{SpiDeviceDriver, SpiDriver},
};
use mipidsi::{models::ST7789, options::{ColorInversion, Orientation, Rotation}, Builder};

/// Display width in pixels (landscape mode - swapped)
#[allow(dead_code)]
pub const DISPLAY_WIDTH: u32 = 240;
/// Display height in pixels (landscape mode - swapped)
#[allow(dead_code)]
pub const DISPLAY_HEIGHT: u32 = 135;

/// Number of display screens available
#[allow(dead_code)]
pub const NUM_SCREENS: u8 = 4;

/// Display screen types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DisplayScreen {
    #[default]
    Status = 0,      // Traffic stats, loop time, errors
    Connection = 1,  // WiFi, MSTP status, baud rate, address
    APConfig = 2,    // WiFi AP mode info (long-press A to activate)
    Splash = 3,      // BACman logo
}

#[allow(dead_code)]
impl DisplayScreen {
    /// Get the next screen in the cycle
    pub fn next(self) -> Self {
        match self {
            DisplayScreen::Status => DisplayScreen::Connection,
            DisplayScreen::Connection => DisplayScreen::APConfig,
            DisplayScreen::APConfig => DisplayScreen::Splash,
            DisplayScreen::Splash => DisplayScreen::Status,
        }
    }

    /// Create from u8 value
    pub fn from_u8(val: u8) -> Self {
        match val % NUM_SCREENS {
            0 => DisplayScreen::Status,
            1 => DisplayScreen::Connection,
            2 => DisplayScreen::APConfig,
            3 => DisplayScreen::Splash,
            _ => DisplayScreen::Status,
        }
    }
}

/// Gateway status for display
#[derive(Clone, Default, PartialEq)]
pub struct GatewayStatus {
    pub wifi_connected: bool,
    pub ip_address: String,
    pub mstp_network: u16,
    pub ip_network: u16,
    pub rx_frames: u64,
    pub tx_frames: u64,
    pub crc_errors: u64,
    pub token_loop_ms: u32,
    pub master_count: u8,
    // Connection screen fields
    pub mstp_address: u8,
    pub mstp_max_master: u8,
    pub mstp_baud_rate: u32,
    pub mstp_state: String,
    pub has_token: bool,
    // AP mode fields
    pub ap_mode_active: bool,
    pub ap_ssid: String,
    pub ap_ip: String,
    pub ap_clients: u8,
}

/// Display wrapper for M5StickC Plus2
#[allow(dead_code)]
pub struct Display<DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    display: mipidsi::Display<SPIInterface<SpiDeviceDriver<'static, SpiDriver<'static>>, PinDriver<'static, DC, esp_idf_svc::hal::gpio::Output>>, ST7789, PinDriver<'static, RST, esp_idf_svc::hal::gpio::Output>>,
    backlight: PinDriver<'static, BL, esp_idf_svc::hal::gpio::Output>,
    /// Track previous status for incremental updates
    last_status: Option<GatewayStatus>,
}

#[allow(dead_code)]
impl<DC, RST, BL> Display<DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    /// Initialize the display
    pub fn new(
        spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
        dc: PinDriver<'static, DC, esp_idf_svc::hal::gpio::Output>,
        rst: PinDriver<'static, RST, esp_idf_svc::hal::gpio::Output>,
        mut backlight: PinDriver<'static, BL, esp_idf_svc::hal::gpio::Output>,
    ) -> Result<Self, anyhow::Error> {
        // Turn on backlight
        backlight.set_high()?;

        // Create SPI interface
        let spi_interface = SPIInterface::new(spi, dc);

        // Initialize display in landscape mode (rotated 90 degrees)
        let mut display = Builder::new(ST7789, spi_interface)
            .display_size(135, 240)  // Physical size before rotation
            .display_offset(52, 40)  // M5StickC Plus2 offset
            .orientation(Orientation::new().rotate(Rotation::Deg90))
            .invert_colors(ColorInversion::Inverted)
            .reset_pin(rst)
            .init(&mut FreeRtos)
            .map_err(|e| anyhow::anyhow!("Display init failed: {:?}", e))?;

        // Clear screen to black
        display.clear(Rgb565::BLACK)
            .map_err(|e| anyhow::anyhow!("Clear failed: {:?}", e))?;

        Ok(Self { display, backlight, last_status: None })
    }

    /// Show splash screen with BACman branding
    pub fn show_splash_screen(&mut self) -> Result<(), anyhow::Error> {
        self.clear()?;

        // Large title - BACman
        let title_style = MonoTextStyle::new(&FONT_10X20, Rgb565::CYAN);
        let subtitle_style = MonoTextStyle::new(&FONT_9X18_BOLD, Rgb565::WHITE);
        let small_style = MonoTextStyle::new(&FONT_6X13, Rgb565::new(20, 40, 20)); // Dark gray

        // Center "BACman" (6 chars * 10px = 60px wide, display is 240px)
        Text::new("BACman", Point::new(90, 55), title_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Subtitle
        Text::new("MS/TP <-> IP Gateway", Point::new(30, 85), subtitle_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Version at bottom
        Text::new("v0.1.0", Point::new(100, 120), small_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Ok(())
    }

    /// Show boot screen (legacy - calls splash screen)
    pub fn show_boot_screen(&mut self) -> Result<(), anyhow::Error> {
        self.show_splash_screen()
    }

    /// Show WiFi connecting screen
    pub fn show_wifi_connecting(&mut self, ssid: &str) -> Result<(), anyhow::Error> {
        self.clear()?;

        let style = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);
        let title_style = MonoTextStyle::new(&FONT_6X13, Rgb565::YELLOW);

        Text::new("WiFi Connecting...", Point::new(50, 50), title_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Truncate SSID if too long (landscape has more room)
        let display_ssid = if ssid.len() > 30 {
            &ssid[..30]
        } else {
            ssid
        };

        Text::new(display_ssid, Point::new(50, 80), style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Ok(())
    }

    /// Draw static elements (title, labels) - called once
    fn draw_static_layout(&mut self) -> Result<(), anyhow::Error> {
        let cyan = MonoTextStyle::new(&FONT_6X13, Rgb565::CYAN);
        let white = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);

        // Title - centered at top
        Text::new("BACnet MS/TP <-> IP Gateway", Point::new(10, 15), cyan)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Static labels
        Text::new("Net:", Point::new(10, 55), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("RX:", Point::new(10, 75), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("TX:", Point::new(100, 75), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("Loop:", Point::new(10, 95), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("Err:", Point::new(100, 95), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("M:", Point::new(170, 95), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Ok(())
    }

    /// Helper to clear a region and draw text
    fn draw_value(&mut self, x: i32, y: i32, width: u32, text: &str, style: MonoTextStyle<'_, Rgb565>) -> Result<(), anyhow::Error> {
        let black_fill = PrimitiveStyle::with_fill(Rgb565::BLACK);

        // Clear the region (y-11 because text baseline is at bottom of character)
        Rectangle::new(Point::new(x, y - 11), Size::new(width, 14))
            .into_styled(black_fill)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Clear failed: {:?}", e))?;

        // Draw new text
        Text::new(text, Point::new(x, y), style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Ok(())
    }

    /// Update status display - only redraws changed values
    pub fn update_status(&mut self, status: &GatewayStatus) -> Result<(), anyhow::Error> {
        let white = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);
        let green = MonoTextStyle::new(&FONT_6X13, Rgb565::GREEN);
        let yellow = MonoTextStyle::new(&FONT_6X13, Rgb565::YELLOW);
        let red = MonoTextStyle::new(&FONT_6X13, Rgb565::RED);

        // First time: draw full layout
        if self.last_status.is_none() {
            self.clear()?;
            self.draw_static_layout()?;

            // Draw all values
            let wifi_text = if status.wifi_connected { "OK" } else { "--" };
            let wifi_style = if status.wifi_connected { green } else { yellow };
            self.draw_value(10, 35, 230, &format!("WiFi:{} {}", wifi_text, status.ip_address), wifi_style)?;

            let net_text = format!("{}<->{}", status.mstp_network, status.ip_network);
            self.draw_value(34, 55, 80, &net_text, white)?;

            self.draw_value(28, 75, 60, &status.rx_frames.to_string(), green)?;
            self.draw_value(118, 75, 70, &status.tx_frames.to_string(), green)?;
            self.draw_value(40, 95, 50, &format!("{}ms", status.token_loop_ms), white)?;

            let err_style = if status.crc_errors > 0 { red } else { green };
            self.draw_value(124, 95, 40, &status.crc_errors.to_string(), err_style)?;
            self.draw_value(182, 95, 30, &status.master_count.to_string(), white)?;

            self.last_status = Some(status.clone());
            return Ok(());
        }

        // Incremental update - only changed fields
        // Clone last_status to avoid borrow checker issues
        let last = self.last_status.clone().unwrap();

        // WiFi status
        if last.wifi_connected != status.wifi_connected || last.ip_address != status.ip_address {
            let wifi_text = if status.wifi_connected { "OK" } else { "--" };
            let wifi_style = if status.wifi_connected { green } else { yellow };
            self.draw_value(10, 35, 230, &format!("WiFi:{} {}", wifi_text, status.ip_address), wifi_style)?;
        }

        // Network numbers (rarely change)
        if last.mstp_network != status.mstp_network || last.ip_network != status.ip_network {
            let net_text = format!("{}<->{}", status.mstp_network, status.ip_network);
            self.draw_value(34, 55, 80, &net_text, white)?;
        }

        // RX frames
        if last.rx_frames != status.rx_frames {
            self.draw_value(28, 75, 60, &status.rx_frames.to_string(), green)?;
        }

        // TX frames
        if last.tx_frames != status.tx_frames {
            self.draw_value(118, 75, 70, &status.tx_frames.to_string(), green)?;
        }

        // Token loop time
        if last.token_loop_ms != status.token_loop_ms {
            self.draw_value(40, 95, 50, &format!("{}ms", status.token_loop_ms), white)?;
        }

        // CRC errors
        if last.crc_errors != status.crc_errors {
            let err_style = if status.crc_errors > 0 { red } else { green };
            self.draw_value(124, 95, 40, &status.crc_errors.to_string(), err_style)?;
        }

        // Master count
        if last.master_count != status.master_count {
            self.draw_value(182, 95, 30, &status.master_count.to_string(), white)?;
        }

        self.last_status = Some(status.clone());
        Ok(())
    }

    /// Clear the display
    pub fn clear(&mut self) -> Result<(), anyhow::Error> {
        self.display.clear(Rgb565::BLACK)
            .map_err(|e| anyhow::anyhow!("Clear failed: {:?}", e))?;
        Ok(())
    }

    /// Clear display and reset status cache (forces full redraw on next update)
    pub fn clear_and_reset(&mut self) -> Result<(), anyhow::Error> {
        self.clear()?;
        self.last_status = None;
        Ok(())
    }

    /// Draw the Connection screen static layout
    fn draw_connection_layout(&mut self) -> Result<(), anyhow::Error> {
        let cyan = MonoTextStyle::new(&FONT_6X13, Rgb565::CYAN);
        let white = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);

        // Title
        Text::new("Connection Status", Point::new(50, 15), cyan)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Static labels
        Text::new("WiFi:", Point::new(10, 35), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("MS/TP:", Point::new(10, 55), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("Baud:", Point::new(10, 75), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("Addr:", Point::new(10, 95), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("Token:", Point::new(10, 115), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Ok(())
    }

    /// Update the Connection screen with current status
    pub fn update_connection(&mut self, status: &GatewayStatus) -> Result<(), anyhow::Error> {
        let green = MonoTextStyle::new(&FONT_6X13, Rgb565::GREEN);
        let yellow = MonoTextStyle::new(&FONT_6X13, Rgb565::YELLOW);
        let red = MonoTextStyle::new(&FONT_6X13, Rgb565::RED);
        let white = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);

        // First time: draw full layout
        if self.last_status.is_none() {
            self.clear()?;
            self.draw_connection_layout()?;

            // WiFi status with IP
            let (wifi_text, wifi_style) = if status.wifi_connected {
                (format!("Connected ({})", status.ip_address), green)
            } else {
                ("Disconnected".to_string(), red)
            };
            self.draw_value(46, 35, 190, &wifi_text, wifi_style)?;

            // MS/TP state
            let mstp_style = if status.has_token { green } else { yellow };
            self.draw_value(50, 55, 180, &status.mstp_state, mstp_style)?;

            // Baud rate
            self.draw_value(46, 75, 100, &format!("{}", status.mstp_baud_rate), white)?;

            // Address info
            let addr_text = format!("{} (max: {})", status.mstp_address, status.mstp_max_master);
            self.draw_value(46, 95, 150, &addr_text, white)?;

            // Token status
            let (token_text, token_style) = if status.has_token {
                ("Have Token", green)
            } else {
                ("Waiting", yellow)
            };
            self.draw_value(50, 115, 100, token_text, token_style)?;

            self.last_status = Some(status.clone());
            return Ok(());
        }

        // Incremental update - only changed fields
        let last = self.last_status.clone().unwrap();

        // WiFi status
        if last.wifi_connected != status.wifi_connected || last.ip_address != status.ip_address {
            let (wifi_text, wifi_style) = if status.wifi_connected {
                (format!("Connected ({})", status.ip_address), green)
            } else {
                ("Disconnected".to_string(), red)
            };
            self.draw_value(46, 35, 190, &wifi_text, wifi_style)?;
        }

        // MS/TP state
        if last.mstp_state != status.mstp_state || last.has_token != status.has_token {
            let mstp_style = if status.has_token { green } else { yellow };
            self.draw_value(50, 55, 180, &status.mstp_state, mstp_style)?;
        }

        // Baud rate (rarely changes)
        if last.mstp_baud_rate != status.mstp_baud_rate {
            self.draw_value(46, 75, 100, &format!("{}", status.mstp_baud_rate), white)?;
        }

        // Address info (rarely changes)
        if last.mstp_address != status.mstp_address || last.mstp_max_master != status.mstp_max_master {
            let addr_text = format!("{} (max: {})", status.mstp_address, status.mstp_max_master);
            self.draw_value(46, 95, 150, &addr_text, white)?;
        }

        // Token status
        if last.has_token != status.has_token {
            let (token_text, token_style) = if status.has_token {
                ("Have Token", green)
            } else {
                ("Waiting", yellow)
            };
            self.draw_value(50, 115, 100, token_text, token_style)?;
        }

        self.last_status = Some(status.clone());
        Ok(())
    }

    /// Draw the AP Config screen static layout
    fn draw_ap_config_layout(&mut self) -> Result<(), anyhow::Error> {
        let cyan = MonoTextStyle::new(&FONT_6X13, Rgb565::CYAN);
        let white = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);

        // Title
        Text::new("WiFi AP Mode", Point::new(70, 15), cyan)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Static labels
        Text::new("Status:", Point::new(10, 35), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("SSID:", Point::new(10, 55), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("IP:", Point::new(10, 75), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Text::new("Clients:", Point::new(10, 95), white)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        // Instruction at bottom
        let small_style = MonoTextStyle::new(&FONT_6X13, Rgb565::new(20, 40, 20)); // Dark gray
        Text::new("Long-press A to toggle", Point::new(40, 125), small_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Draw failed: {:?}", e))?;

        Ok(())
    }

    /// Update the AP Config screen with current status
    pub fn update_ap_config(&mut self, status: &GatewayStatus) -> Result<(), anyhow::Error> {
        let green = MonoTextStyle::new(&FONT_6X13, Rgb565::GREEN);
        let yellow = MonoTextStyle::new(&FONT_6X13, Rgb565::YELLOW);
        let white = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);

        // First time: draw full layout
        if self.last_status.is_none() {
            self.clear()?;
            self.draw_ap_config_layout()?;

            // AP mode status
            let (status_text, status_style) = if status.ap_mode_active {
                ("ACTIVE", green)
            } else {
                ("Inactive", yellow)
            };
            self.draw_value(58, 35, 100, status_text, status_style)?;

            // SSID
            let ssid_display = if status.ap_ssid.len() > 18 {
                &status.ap_ssid[..18]
            } else {
                &status.ap_ssid
            };
            self.draw_value(46, 55, 180, ssid_display, white)?;

            // IP Address
            let ip_text = if status.ap_mode_active {
                &status.ap_ip
            } else {
                "192.168.4.1"
            };
            self.draw_value(28, 75, 150, ip_text, white)?;

            // Connected clients
            let clients_text = if status.ap_mode_active {
                format!("{}", status.ap_clients)
            } else {
                "-".to_string()
            };
            self.draw_value(64, 95, 50, &clients_text, white)?;

            self.last_status = Some(status.clone());
            return Ok(());
        }

        // Incremental update - only changed fields
        let last = self.last_status.clone().unwrap();

        // AP mode status
        if last.ap_mode_active != status.ap_mode_active {
            let (status_text, status_style) = if status.ap_mode_active {
                ("ACTIVE", green)
            } else {
                ("Inactive", yellow)
            };
            self.draw_value(58, 35, 100, status_text, status_style)?;

            // Also update IP when mode changes
            let ip_text = if status.ap_mode_active {
                &status.ap_ip
            } else {
                "192.168.4.1"
            };
            self.draw_value(28, 75, 150, ip_text, white)?;
        }

        // SSID (rarely changes)
        if last.ap_ssid != status.ap_ssid {
            let ssid_display = if status.ap_ssid.len() > 18 {
                &status.ap_ssid[..18]
            } else {
                &status.ap_ssid
            };
            self.draw_value(46, 55, 180, ssid_display, white)?;
        }

        // IP address
        if last.ap_ip != status.ap_ip {
            let ip_text = if status.ap_mode_active {
                &status.ap_ip
            } else {
                "192.168.4.1"
            };
            self.draw_value(28, 75, 150, ip_text, white)?;
        }

        // Connected clients
        if last.ap_clients != status.ap_clients || last.ap_mode_active != status.ap_mode_active {
            let clients_text = if status.ap_mode_active {
                format!("{}", status.ap_clients)
            } else {
                "-".to_string()
            };
            self.draw_value(64, 95, 50, &clients_text, white)?;
        }

        self.last_status = Some(status.clone());
        Ok(())
    }

    /// Turn backlight on
    pub fn backlight_on(&mut self) -> Result<(), anyhow::Error> {
        self.backlight.set_high()?;
        Ok(())
    }

    /// Turn backlight off
    pub fn backlight_off(&mut self) -> Result<(), anyhow::Error> {
        self.backlight.set_low()?;
        Ok(())
    }
}

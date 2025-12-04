use pyo3::prelude::*;
use rxing::BarcodeFormat;
use image::{DynamicImage, GenericImageView};

/// QR tarama sonucunu döndüren yardımcı fonksiyon (Raw Luma)
fn scan_helper_raw(width: u32, height: u32, raw_pixels: Vec<u8>) -> Option<String> {
    // rxing helper fonksiyonunu çağır
    match rxing::helpers::detect_in_luma(raw_pixels, width, height, Some(BarcodeFormat::QR_CODE)) {
        Ok(result) => Some(result.getText().to_string()),
        Err(_) => None,
    }
}

/// QR tarama sonucunu döndüren yardımcı fonksiyon (DynamicImage wrapper)
fn scan_helper(img: &DynamicImage) -> Option<String> {
    let width = img.width();
    let height = img.height();
    
    // Gri tonlamaya (Luma) çevir ve vektöre dönüştür
    let luma_img = img.to_luma8();
    let raw_pixels = luma_img.into_vec();

    scan_helper_raw(width, height, raw_pixels)
}

/// Ham Luma (Gri Tonlama) verisini alıp QR arar (Performans için)
#[pyfunction]
fn scan_raw_luma(data: &[u8], width: u32, height: u32) -> PyResult<Option<String>> {
    // 1. Try raw scan first (fastest)
    let raw_pixels = data.to_vec();
    if let Some(qr) = scan_helper_raw(width, height, raw_pixels.clone()) {
         return Ok(Some(qr));
    }

    // 2. Convert to DynamicImage for advanced ops (Retry Logic)
    let img_buffer = match image::ImageBuffer::<image::Luma<u8>, _>::from_raw(width, height, raw_pixels) {
        Some(buf) => buf,
        None => return Ok(None),
    };
    let img = DynamicImage::ImageLuma8(img_buffer);

    // --- AŞAMA 2: Sağ Üst Köşe (Top-Right Crop) ---
    let crop_x = (width as f32 * 0.60) as u32;
    let crop_w = width - crop_x;
    let crop_h = (height as f32 * 0.40) as u32;

    let cropped_img = img.crop_imm(crop_x, 0, crop_w, crop_h);
    if let Some(qr) = scan_helper(&cropped_img) {
        return Ok(Some(qr));
    }

    // --- AŞAMA 3: Derin Tarama (Kontrast Artırma) ---
    let mut gray_img = img.to_luma8();
    image::imageops::contrast(&mut gray_img, 20.0);
    let enhanced_img = DynamicImage::ImageLuma8(gray_img);
    
    if let Some(qr) = scan_helper(&enhanced_img) {
        return Ok(Some(qr));
    }

    Ok(None)
}

/// Görüntü baytlarını (bytes) alır ve QR arar
#[pyfunction]
fn scan_image_bytes(data: &[u8]) -> PyResult<Option<String>> {
    // 1. Görüntüyü yükle (PNG, JPG vb. formatını otomatik algılar)
    let img = match image::load_from_memory(data) {
        Ok(i) => i,
        Err(_) => return Ok(None), // Görüntü bozuksa None dön
    };

    // --- AŞAMA 1: Hızlı Tarama (Tam Resim) ---
    if let Some(qr) = scan_helper(&img) {
        return Ok(Some(qr));
    }

    // --- AŞAMA 2: Sağ Üst Köşe ---
    let (w, h) = img.dimensions();
    let crop_x = (w as f32 * 0.60) as u32;
    let crop_w = w - crop_x;
    let crop_h = (h as f32 * 0.40) as u32;

    let cropped_img = img.crop_imm(crop_x, 0, crop_w, crop_h);
    if let Some(qr) = scan_helper(&cropped_img) {
        return Ok(Some(qr));
    }

    // --- AŞAMA 3: Derin Tarama (Kontrast Artırma) ---
    let mut gray_img = img.to_luma8();
    
    // Kontrast germe işlemi
    image::imageops::contrast(&mut gray_img, 20.0);
    
    let enhanced_img = DynamicImage::ImageLuma8(gray_img);
    if let Some(qr) = scan_helper(&enhanced_img) {
        return Ok(Some(qr));
    }

    Ok(None)
}

/// JSON Temizleme Fonksiyonu
#[pyfunction]
fn clean_json_string(text: String) -> PyResult<String> {
    let cleaned: String = text.chars()
        .filter(|&c| !c.is_control())
        .collect();
    
    let cleaned = cleaned.replace("\\x", "")
                         .replace("'", "\"")
                         .replace("“", "\"")
                         .replace("”", "\"");
    
    Ok(cleaned)
}

/// Modül Tanımlaması (PyO3 0.21+ Bound Syntax)
#[pymodule]
fn rust_qr_backend(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_image_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(clean_json_string, m)?)?;
    m.add_function(wrap_pyfunction!(scan_raw_luma, m)?)?;
    Ok(())
}
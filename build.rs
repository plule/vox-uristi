fn main() {
    #[cfg(windows)]
    add_resources();

    gen_icon_data();
}

#[cfg(windows)]
fn add_resources() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("site/assets/icon.ico");
    res.compile().unwrap();
}

fn gen_icon_data() {
    println!("cargo:rerun-if-changed=site/assets/icon.png");
    const ICON: &[u8] = include_bytes!("site/assets/icon.png");
    let image = image::load_from_memory_with_format(ICON, image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();
    let rgba = image.into_raw();
    let dst = format!("{}/icon", std::env::var("OUT_DIR").unwrap());
    std::fs::write(dst, rgba).unwrap();
}

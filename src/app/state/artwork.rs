use image::DynamicImage;

pub enum ArtworkState {
    Idle,
    Loading,
    Loaded(DynamicImage),
    Failed,
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlobalsUniform {
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub time: f32,
    pub delta_time: f32,
    pub frame: u32,
    pub fps: f32,
}

impl GlobalsUniform {
    pub fn new(width: u32, height: u32, frame: u32, fps: f32) -> Self {
        let w = width as f32;
        let h = height as f32;
        let delta_time = if fps > 0.0 { 1.0 / fps } else { 1.0 / 30.0 };
        let time = frame as f32 * delta_time;

        Self {
            resolution: [w, h],
            inv_resolution: [1.0 / w, 1.0 / h],
            time,
            delta_time,
            frame,
            fps,
        }
    }
}

use cgmath::Matrix4;

#[derive(Debug, Clone)]
pub struct UniformBufferObject {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    projection: Matrix4<f32>
}



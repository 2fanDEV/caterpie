use cgmath::Matrix4;

pub struct UniformBufferObject {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    projection: Matrix4<f32>
}



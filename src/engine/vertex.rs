use std::mem::offset_of;

use ash::vk::{
    Format, VertexInputAttributeDescription, VertexInputBindingDescription, VertexInputRate,
};
use cgmath::{Vector2, Vector3};


#[derive(Debug, Clone)]
pub struct Vertex {
    pos: Vector2<f32>,
    color: Vector3<f32>,
}

impl Vertex {

    pub fn new(pos: Vector2<f32>, color: Vector3<f32>) -> Self {
                Vertex {pos,  color}
    }

    pub fn get_binding_description() -> Vec<VertexInputBindingDescription> {
        return vec![VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(VertexInputRate::VERTEX)];
    }

    pub fn get_attribute_description() -> Vec<VertexInputAttributeDescription> {
        let mut attribute_descriptons: [VertexInputAttributeDescription; 2] =
            [Default::default(); 2];
        attribute_descriptons[0] = attribute_descriptons[0]
            .binding(0)
            .location(0)
            .format(Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex, pos) as u32);


        attribute_descriptons[1] = attribute_descriptons[1]
            .binding(0)
            .location(1)
            .format(Format::R32G32B32_SFLOAT)
            .offset(offset_of!(Vertex, color) as u32);

        attribute_descriptons.to_vec()
    }
}

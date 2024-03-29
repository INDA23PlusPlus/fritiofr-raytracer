mod mesh;
mod plane;
mod sphere;
mod triangle;

pub enum Obj {
    Sphere(Sphere),
    Plane(Plane),
    Triangle(Triangle),
    Mesh(Mesh),
}

pub trait ToObj {
    fn to_obj(&self) -> Obj;
}

pub use mesh::Mesh;
pub use plane::Plane;
pub use sphere::Sphere;
pub use triangle::Triangle;

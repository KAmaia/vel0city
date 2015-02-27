use bsp;

pub struct Map {
    pub bsp: bsp::Tree
}

#[cfg(test)]
pub mod test {
    use bsp;
    use na;
    use super::Map;

    pub fn single_plane_map() -> Map {
        use bsp::{Tree, Plane};
        use bsp::{InnerNode, Leaf};

        Map {
            bsp: Tree {
                inodes: vec![
                    InnerNode {
                        plane: Plane {
                            norm: na::Vec3::new(0.0, 1.0, 0.0),
                            dist: 0.0
                        },
                        pos: -1,
                        neg: -2
                    },
                ],
                leaves: vec![
                    Leaf { solid: false },
                    Leaf { solid: true }
                ],
                root: 0
            }
        }
    }
}
